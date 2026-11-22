from volatility3.framework import interfaces, renderers
from volatility3.framework.configuration import requirements


class CpuState(interfaces.plugins.PluginInterface):
    _required_framework_version = (2, 0, 0)

    @classmethod
    def get_requirements(cls):
        return [requirements.ModuleRequirement(name="kernel", architectures=["Intel32", "Intel64"])]

    def _run(self):
        kernel = self.context.modules[self.config["kernel"]]
        n = 4; layer = self.context.layers[kernel.layer_name]
        base = kernel.offset + kernel.get_symbol("KiProcessorBlock").address
        sym = kernel.symbol_table_name
        for i in range(n):
            prcb_ptr = int.from_bytes(layer.read(base + 8 * i, 8), "little")
            prcb = self.context.object(sym + "!_KPRCB", kernel.layer_name, prcb_ptr)
            vals = []
            for field in ["Number", "InterruptCount", "DpcCount", "DpcRoutineActive", "DpcInterruptRequested",
                          "TimerRequest", "TickOffset", "ClockInterruptCount", "DpcTime", "InterruptTime",
                          "KernelTime", "UserTime", "IdleTime"]:
                try:
                    vals.append("%s=%s" % (field, int(getattr(prcb, field))))
                except Exception:
                    pass
            for ptr in ["CurrentThread", "NextThread", "IdleThread"]:
                try:
                    vals.append("%s=%#x" % (ptr, int(getattr(prcb, ptr))))
                except Exception:
                    pass
            try:
                dq = prcb.DpcData[0]
                vals.append("DpcQueueDepth=%d" % int(dq.DpcQueueDepth))
                vals.append("DpcCountData=%d" % int(dq.DpcCount))
            except Exception:
                pass
            try:
                kpcr = self.context.object(sym + "!_KPCR", kernel.layer_name, prcb_ptr - 0x180)
                vals.append("Irql=%d" % int(kpcr.Irql))
            except Exception as e:
                vals.append("Irql=?(%s)" % e)
            yield (0, (i, " ".join(vals)))

    def run(self):
        return renderers.TreeGrid([("CPU", int), ("State", str)], self._run())
