typedef signed int int32_t;
typedef __int64 intptr_t;
typedef unsigned char uint8_t;

static void copy_bytes(void *destination, const void *source, unsigned long count) {
    unsigned char *out = (unsigned char *)destination;
    const unsigned char *in = (const unsigned char *)source;
    for (unsigned long index = 0; index < count; ++index) {
        out[index] = in[index];
    }
}

static void clear_bytes(void *destination, unsigned long count) {
    unsigned char *out = (unsigned char *)destination;
    for (unsigned long index = 0; index < count; ++index) {
        out[index] = 0;
    }
}

typedef struct AEffect AEffect;
typedef intptr_t (*audio_master_callback)(AEffect *, int32_t, int32_t, intptr_t,
                                           void *, float);
typedef intptr_t (*dispatcher_proc)(AEffect *, int32_t, int32_t, intptr_t,
                                    void *, float);
typedef void (*process_proc)(AEffect *, const float *const *, float **, int32_t);
typedef void (*set_parameter_proc)(AEffect *, int32_t, float);
typedef float (*get_parameter_proc)(AEffect *, int32_t);

struct AEffect {
    int32_t magic;
    dispatcher_proc dispatcher;
    process_proc process;
    set_parameter_proc set_parameter;
    get_parameter_proc get_parameter;
    int32_t num_programs;
    int32_t num_parameters;
    int32_t num_inputs;
    int32_t num_outputs;
    int32_t flags;
    intptr_t reserved_1;
    intptr_t reserved_2;
    int32_t initial_delay;
    int32_t real_quality;
    int32_t off_quality;
    float io_ratio;
    void *object;
    void *user;
    int32_t unique_id;
    int32_t version;
    process_proc process_replacing;
    uint8_t future[56];
};

enum {
    EFF_OPEN = 0,
    EFF_CLOSE = 1,
    EFF_GET_PARAM_NAME = 8,
    EFF_SET_SAMPLE_RATE = 10,
    EFF_SET_BLOCK_SIZE = 11,
    EFF_MAINS_CHANGED = 12,
    EFF_GET_CHUNK = 23,
    EFF_SET_CHUNK = 24,
};

static AEffect effect;
static float mix = 0.5f;

static intptr_t dispatch(AEffect *self, int32_t opcode, int32_t index,
                         intptr_t value, void *ptr, float opt) {
    (void)self;
    (void)index;
    (void)value;
    (void)opt;
    switch (opcode) {
    case EFF_OPEN:
    case EFF_CLOSE:
    case EFF_SET_SAMPLE_RATE:
    case EFF_SET_BLOCK_SIZE:
    case EFF_MAINS_CHANGED:
        return 0;
    case EFF_GET_PARAM_NAME:
        if (ptr != 0) {
            const char *name = "Mix";
            copy_bytes(ptr, name, 4);
        }
        return 1;
    case EFF_GET_CHUNK:
        if (ptr != 0) {
            *(void **)ptr = &mix;
        }
        return (intptr_t)sizeof(mix);
    case EFF_SET_CHUNK:
        if (ptr != 0 && value == (intptr_t)sizeof(mix)) {
            copy_bytes(&mix, ptr, sizeof(mix));
            return 1;
        }
        return 0;
    default:
        return 0;
    }
}

static void process_replacing(AEffect *self, const float *const *inputs,
                              float **outputs, int32_t frames) {
    (void)self;
#ifdef VST2_NONFINITE_OUTPUT
    (void)inputs;
#endif
#if defined(VST2_CRASH_OUTPUT) || defined(VST2_HANG_OUTPUT)
    (void)inputs;
    (void)outputs;
#endif
    for (int32_t frame = 0; frame < frames; ++frame) {
#ifdef VST2_CRASH_OUTPUT
        (void)frame;
        *(volatile int *)0 = 1;
#elif defined(VST2_HANG_OUTPUT)
        (void)frame;
        volatile int keep_running = 1;
        while (keep_running) {
        }
#elif defined(VST2_NONFINITE_OUTPUT)
        volatile float zero = 0.0f;
        outputs[0][frame] = 0.0f / zero;
        outputs[1][frame] = 0.0f / zero;
#else
        outputs[0][frame] = inputs[0][frame] * mix;
        outputs[1][frame] = inputs[1][frame] * mix;
#endif
    }
}

static void set_parameter(AEffect *self, int32_t index, float value) {
    (void)self;
    if (index == 0 && value >= 0.0f && value <= 1.0f) {
        mix = value;
    }
}

static float get_parameter(AEffect *self, int32_t index) {
    (void)self;
    return index == 0 ? mix : 0.0f;
}

#ifdef LEGACY_VST2_MAIN
__declspec(dllexport) AEffect *main(audio_master_callback callback) {
#else
__declspec(dllexport) AEffect *VSTPluginMain(audio_master_callback callback) {
#endif
    (void)callback;
    clear_bytes(&effect, sizeof(effect));
    effect.magic = 0x56737450;
    effect.dispatcher = dispatch;
    effect.set_parameter = set_parameter;
    effect.get_parameter = get_parameter;
    effect.num_programs = 1;
    effect.num_parameters = 1;
    effect.num_inputs = 2;
    effect.num_outputs = 2;
    effect.flags = (1 << 4) | (1 << 5);
    effect.io_ratio = 1.0f;
    effect.unique_id = 0x41525354;
    effect.version = 1;
    effect.process_replacing = process_replacing;
    return &effect;
}
