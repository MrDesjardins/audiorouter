from volatility3.framework import interfaces, renderers, symbols, exceptions
from volatility3.framework.configuration import requirements
from volatility3.plugins.windows import pslist, modules

STATES = {0: "Init", 1: "Ready", 2: "Running", 3: "Standby", 4: "Terminated",
          5: "Waiting", 6: "Transition", 7: "DeferredReady", 8: "GateWait"}


class ThreadState(interfaces.plugins.PluginInterface):
    _required_framework_version = (2, 0, 0)

    @classmethod
    def get_requirements(cls):
        return [
            requirements.ModuleRequirement(name="kernel", architectures=["Intel32", "Intel64"]),
            requirements.VersionRequirement(name="pslist", component=pslist.PsList, version=(3, 0, 0)),
            requirements.ListRequirement(name="pids", element_type=int, optional=True),
            requirements.BooleanRequirement(name="stacks", optional=True),
        ]

    def _mods(self, kernel):
        res = []
        for m in modules.Modules.list_modules(self.context, self.config["kernel"]):
            try:
                res.append((m.DllBase, m.DllBase + m.SizeOfImage, m.BaseDllName.get_string()))
            except Exception:
                pass
        return res

    def _run(self):
        kernel = self.context.modules[self.config["kernel"]]
        layer = self.context.layers[kernel.layer_name]
        mods = self._mods(kernel)
        pids = self.config.get("pids")
        stacks = self.config.get("stacks")
        for proc in pslist.PsList.list_processes(self.context, self.config["kernel"]):
            pid = int(proc.UniqueProcessId)
            if pids and pid not in pids:
                continue
            name = proc.ImageFileName.cast("string", max_length=15, errors="replace")
            try:
                threads = list(proc.ThreadListHead.to_list(kernel.symbol_table_name + "!_ETHREAD", "ThreadListEntry"))
            except Exception:
                continue
            for t in threads:
                try:
                    st = int(t.Tcb.State)
                    wr = int(t.Tcb.WaitReason)
                    wt = int(t.Tcb.WaitTime)
                    tid = int(t.Cid.UniqueThread)
                    ks = int(t.Tcb.KernelStack); ks = ks | (0xffff << 48) if (ks >> 47) & 1 else ks
                    sa = int(t.Win32StartAddress) if hasattr(t, "Win32StartAddress") else 0
                except Exception:
                    continue
                line = "%s pid=%d tid=%d state=%s wr=%d waittime=%d ks=%#x" % (name, pid, tid, STATES.get(st, st), wr, wt, ks)
                chain = []
                if stacks and ks:
                    try:
                        data = layer.read(ks, 0x1800, pad=True)
                        for i in range(0, len(data) - 7, 8):
                            q = int.from_bytes(data[i:i + 8], "little") & 0xffffffffffff
                            for lo, hi, mn in mods:
                                if lo <= q < hi:
                                    chain.append("%s+%#x" % (mn, q - lo))
                                    break
                    except Exception:
                        pass
                yield (0, (line, " ".join(chain[:40])))

    def run(self):
        return renderers.TreeGrid([("Thread", str), ("Stack modules", str)], self._run())
