/*++

  Bounded PCM conversion helpers shared by the WaveRT callback and host tests.
  PCM24 is represented in a 32-bit container with valid bits left-aligned.

--*/
#ifndef _AUDIOROUTERVIRTUAL_SAMPLECONV_H_
#define _AUDIOROUTERVIRTUAL_SAMPLECONV_H_

#define AR_CABLE_FORMAT_PCM 1
#define AR_CABLE_FORMAT_FLOAT32 2

// Keep the portable acceptance inventory alongside conversion logic. The
// WDK tables use the same finite set (5 channel layouts x 3 rates x 4 forms).
__forceinline bool AudioRouterCableFormatSupported(
    unsigned long SampleRateHz, unsigned short Channels, unsigned long Encoding,
    unsigned short ContainerBits, unsigned short ValidBits)
{
    if (SampleRateHz != 44100 && SampleRateHz != 48000 && SampleRateHz != 96000) {
        return false;
    }
    if (Channels != 1 && Channels != 2 && Channels != 4 && Channels != 6 && Channels != 8) {
        return false;
    }
    if (Encoding == AR_CABLE_FORMAT_FLOAT32) {
        return ContainerBits == 32 && ValidBits == 32;
    }
    return Encoding == AR_CABLE_FORMAT_PCM &&
        ((ContainerBits == 16 && ValidBits == 16) ||
         (ContainerBits == 32 && (ValidBits == 24 || ValidBits == 32)));
}

__forceinline bool AudioRouterDoubleIsFinite(DOUBLE Value)
{
    return Value == Value && Value <= 1.7976931348623157e+308 &&
        Value >= -1.7976931348623157e+308;
}

__forceinline DOUBLE AudioRouterFloat32ToDouble(FLOAT Value)
{
    return Value == Value && Value <= 3.402823466e+38F &&
        Value >= -3.402823466e+38F ? static_cast<DOUBLE>(Value) : 0.0;
}

__forceinline FLOAT AudioRouterDoubleToFloat32(DOUBLE Value)
{
    return AudioRouterDoubleIsFinite(Value) &&
        Value <= 3.402823466e+38 && Value >= -3.402823466e+38
        ? static_cast<FLOAT>(Value) : 0.0F;
}

__forceinline DOUBLE AudioRouterPcm16ToDouble(SHORT Value)
{
    return static_cast<DOUBLE>(Value) / 32768.0;
}

__forceinline DOUBLE AudioRouterPcm24In32ToDouble(LONG Value)
{
    return static_cast<DOUBLE>(Value >> 8) / 8388608.0;
}

__forceinline DOUBLE AudioRouterPcm32ToDouble(LONG Value)
{
    return static_cast<DOUBLE>(Value) / 2147483648.0;
}

__forceinline DOUBLE AudioRouterClampUnitDouble(DOUBLE Value)
{
    if (Value != Value) { return 0.0; }
    if (Value <= -1.0) { return -1.0; }
    if (Value >= 1.0) { return 1.0; }
    return Value;
}

__forceinline SHORT AudioRouterDoubleToPcm16(DOUBLE Sample)
{
    Sample = AudioRouterClampUnitDouble(Sample);
    if (Sample <= -1.0) { return static_cast<SHORT>(-32768); }
    if (Sample >= 1.0) { return static_cast<SHORT>(32767); }
    DOUBLE scaled = Sample * 32768.0;
    LONG rounded = static_cast<LONG>(scaled >= 0.0 ? scaled + 0.5 : scaled - 0.5);
    if (rounded < -32768) { rounded = -32768; }
    if (rounded > 32767) { rounded = 32767; }
    return static_cast<SHORT>(rounded);
}

__forceinline LONG AudioRouterDoubleToPcm24In32(DOUBLE Sample)
{
    Sample = AudioRouterClampUnitDouble(Sample);
    if (Sample <= -1.0) { return static_cast<LONG>(-2147483647L - 1L); }
    if (Sample >= 1.0) { return static_cast<LONG>(2147483392L); }
    DOUBLE scaled = Sample * 8388608.0;
    LONG rounded = static_cast<LONG>(scaled >= 0.0 ? scaled + 0.5 : scaled - 0.5);
    if (rounded < -8388608) { rounded = -8388608; }
    if (rounded > 8388607) { rounded = 8388607; }
    return rounded * 256;
}

__forceinline LONG AudioRouterDoubleToPcm32(DOUBLE Sample)
{
    Sample = AudioRouterClampUnitDouble(Sample);
    if (Sample <= -1.0) { return static_cast<LONG>(-2147483647L - 1L); }
    if (Sample >= 1.0) { return static_cast<LONG>(2147483647L); }
    DOUBLE scaled = Sample * 2147483648.0;
    if (scaled >= 2147483647.0) { return static_cast<LONG>(2147483647L); }
    LONG rounded = static_cast<LONG>(scaled >= 0.0 ? scaled + 0.5 : scaled - 0.5);
    return rounded;
}

#endif
