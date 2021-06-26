# iced-cpuid [![GitHub builds](https://github.com/icedland/iced-cpuid/workflows/GitHub%20CI/badge.svg)](https://github.com/icedland/iced-cpuid/actions)

Shows CPUID features and instruction encodings used by x86/x64 binaries.

It assumes all bytes inside the code sections are code. This isn't always true.
If you see some weird instructions, it's probably data that was decoded as instructions.

## Prerequisites

- Rust: https://www.rust-lang.org/tools/install

## Build

```sh
cargo install iced-cpuid
```

## Example usage

Show all CPUID features `/usr/bin/gcc` uses:

```sh
$ iced-cpuid /usr/bin/gcc
CET_IBT
CMOV
SSE
SSE2
XSAVE
```

Show instructions too:

```sh
$ iced-cpuid /usr/bin/gcc -i
CET_IBT
	ENDBR64
CMOV
	CMOVA r32, r/m32
	CMOVA r64, r/m64
	CMOVAE r32, r/m32
[...]
SSE
	MOVAPS xmm1, xmm2/m128
	MOVAPS xmm2/m128, xmm1
	MOVHPS xmm1, m64
[...]
```

Show instructions, opcodes, and count:

```sh
$ iced-cpuid /usr/bin/gcc -ioc
CET_IBT
	788 | F3 0F 1E FA | ENDBR64
CMOV
	5 | o32 0F 47 /r | CMOVA r32, r/m32
	53 | o64 0F 47 /r | CMOVA r64, r/m64
	1 | o32 0F 43 /r | CMOVAE r32, r/m32
[...]
SSE
	1 | NP 0F 28 /r | MOVAPS xmm1, xmm2/m128
	278 | NP 0F 29 /r | MOVAPS xmm2/m128, xmm1
	11 | NP 0F 16 /r | MOVHPS xmm1, m64
[...]
```

Show only `SSE` and `SSE2` instructions:

```sh
$ iced-cpuid /usr/bin/gcc -i --cpuid SSE,SSE2
SSE
	MOVAPS xmm1, xmm2/m128
	MOVAPS xmm2/m128, xmm1
	MOVHPS xmm1, m64
	MOVUPS xmm1, xmm2/m128
	MOVUPS xmm2/m128, xmm1
	PREFETCHNTA m8
	PREFETCHT0 m8
	XORPS xmm1, xmm2/m128
SSE2
	ADDSD xmm1, xmm2/m64
	COMISD xmm1, xmm2/m64
	CVTSI2SD xmm1, r/m64
	DIVSD xmm1, xmm2/m64
	MOVAPD xmm1, xmm2/m128
	MOVD xmm, r/m32
[...]
```

## CPUID feature names

The CPUID feature name strings shown in the output and used as input (`--cpuid XXX`) are identical to the enum variants in [`iced-x86`](https://github.com/icedland/iced), see its source code or https://docs.rs/iced-x86/ (search for `CpuidFeature`).

## Similar programs

- elfx86exts (uses capstone) https://crates.io/crates/elfx86exts