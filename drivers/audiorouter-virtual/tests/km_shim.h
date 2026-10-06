#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>
using NTSTATUS = std::int32_t;
using UCHAR = unsigned char;
using USHORT = std::uint16_t;
using SHORT = std::int16_t;
using ULONG = std::uint32_t;
using LONG = std::int32_t;
using ULONGLONG = std::uint64_t;
using SIZE_T = std::size_t;
using WCHAR = wchar_t;
using FLOAT = float;
using DOUBLE = double;
#ifndef _In_
#define _In_
#define _Out_
#define _Out_writes_(n)
#define _In_reads_(n)
#endif
#define NT_SUCCESS(status) ((status) >= 0)
#define STATUS_SUCCESS NTSTATUS(0)
#define STATUS_INVALID_PARAMETER NTSTATUS(0xC000000D)
#define STATUS_BUFFER_TOO_SMALL NTSTATUS(0xC0000023)
#define STATUS_DATA_ERROR NTSTATUS(0xC000003E)
#define STATUS_REVISION_MISMATCH NTSTATUS(0xC0000059)
#define STATUS_SHARING_VIOLATION NTSTATUS(0xC0000043)
#define STATUS_ACCESS_DENIED NTSTATUS(0xC0000022)
#define FIELD_OFFSET(type, member) offsetof(type, member)
#define C_ASSERT(expression) static_assert(expression, #expression)
#define ARRAYSIZE(array) (sizeof(array) / sizeof(array[0]))
#define RtlCopyMemory(destination, source, bytes) std::memcpy(destination, source, bytes)
#define RtlZeroMemory(destination, bytes) std::memset(destination, 0, bytes)
#define DECLARE_CONST_UNICODE_STRING(name, value) const WCHAR name[] = value
#define FILE_DEVICE_UNKNOWN 0x22
#define METHOD_BUFFERED 0
#define FILE_READ_DATA 1
#define FILE_WRITE_DATA 2
#define CTL_CODE(device, function, method, access) (((device) << 16) | ((access) << 14) | ((function) << 2) | (method))
void AudioRouterTestAfterHeaderSnapshot(const UCHAR* view);
