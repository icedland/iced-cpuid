// SPDX-License-Identifier: MIT
// Copyright (C) 2021-present https://github.com/icedland

use anyhow::Context;
use hashbrown::HashMap;
use iced_x86::{Code, CpuidFeature, Decoder, DecoderOptions, Instruction};
use memmap::Mmap;
use object::{File, Object, ObjectSection, SectionKind};
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

/// Shows CPUID features and instruction encodings used by x86/x64 binaries
///
/// It assumes all bytes inside the code sections are code. This isn't always true.
/// If you see some weird instructions, it's probably data that was decoded as instructions.
#[derive(StructOpt)]
#[structopt(about)]
struct CommandLineOptions {
	#[structopt(parse(from_os_str), help = "The executable to decode")]
	filename: PathBuf,

	#[structopt(long, help = "Decodes MPX instructions")]
	mpx: bool,

	#[structopt(short = "a", long, help = "Includes all instructions even if they don't have a CPUID feature bit")]
	all: bool,

	#[structopt(short = "i", long, help = "Shows instructions")]
	instr: bool,

	#[structopt(short = "o", long, help = "Shows opcodes")]
	opcode: bool,

	#[structopt(short = "c", long, help = "Shows instruction count (requires --instr or --opcode)")]
	count: bool,

	#[structopt(short = "%", long, help = "Shows how often an instruction is used (%) (requires --instr or --opcode)")]
	percent: bool,

	#[structopt(long, help = "Shows only the following CPUID features (','-separated). Matches whole strings.")]
	cpuid: Option<String>,

	#[structopt(long = "ignore-cpuid", help = "Ignores the following CPUID features (','-separated). Matches whole strings.")]
	ignore_cpuid: Option<String>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
struct CodeInfo {
	code: Code,
	count: usize,
}

#[derive(Default)]
struct CpuidInfo {
	codes: HashMap<Code, CodeInfo>,
}

fn main() -> anyhow::Result<()> {
	let cmd = CommandLineOptions::from_args();
	let show_more_info = cmd.instr || cmd.opcode;

	let data = fs::File::open(&cmd.filename).with_context(|| format!("Couldn't open `{}`", cmd.filename.to_string_lossy()))?;
	let mmap = unsafe { Mmap::map(&data)? };
	let file = File::parse(&*mmap).with_context(|| format!("Couldn't read `{}`", cmd.filename.to_string_lossy()))?;

	let mut all_cpuid1: Vec<(CpuidFeature, CpuidInfo)> = CpuidFeature::values().map(|f| (f, CpuidInfo::default())).collect();
	let mut all_cpuidn: HashMap<Vec<CpuidFeature>, CpuidInfo> = HashMap::new();
	let bitness = if file.is_64() { 64 } else { 32 };
	let mut total_instrs = 0;
	for section in file.sections().filter(|s| s.kind() == SectionKind::Text) {
		let decoder_options = if cmd.mpx { DecoderOptions::MPX } else { DecoderOptions::NONE };
		let section_data = section
			.data()
			.with_context(|| format!("Couldn't get section data, section `{}` index {}", section.name().unwrap_or_default(), section.index().0))?;
		let section_address = file.relative_address_base() + section.address();
		let mut decoder = Decoder::with_ip(bitness, section_data, section_address, decoder_options);
		let mut instr = Instruction::default();
		while decoder.can_decode() {
			total_instrs += 1;
			decoder.decode_out(&mut instr);
			// Some instructions require multiple CPUID features eg. 'AES and AVX', but if all we're doing
			// is showing the CPUID feature names, don't show 'xx and yy'
			let cpuid_features = instr.cpuid_features();
			if !show_more_info {
				for &cpuid in cpuid_features {
					all_cpuid1[cpuid as usize].1.codes.entry(instr.code()).or_insert(CodeInfo { code: instr.code(), count: 0 }).count += 1;
				}
			} else if cpuid_features.len() == 1 {
				all_cpuid1[cpuid_features[0] as usize].1.codes.entry(instr.code()).or_insert(CodeInfo { code: instr.code(), count: 0 }).count += 1;
			} else {
				all_cpuidn
					.entry(cpuid_features.to_vec())
					.or_default()
					.codes
					.entry(instr.code())
					.or_insert(CodeInfo { code: instr.code(), count: 0 })
					.count += 1;
			}
		}
	}

	let mut all_cpuid: Vec<_> = all_cpuidn
		.into_iter()
		.chain(all_cpuid1.into_iter().map(|(feature, info)| (vec![feature], info)))
		.filter_map(|(cpuid, info)| {
			if info.codes.is_empty() {
				None
			} else {
				Some((cpuid.iter().map(|&a| format!("{:?}", a)).collect::<Vec<String>>().join(" and "), cpuid, info))
			}
		})
		.collect();
	all_cpuid.sort_unstable_by_key(|e| e.0.clone());
	fn to_cpuid_filter_vec(cpuid: Option<String>) -> Vec<String> {
		cpuid.unwrap_or_default().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()
	}
	let cpuid_filter = to_cpuid_filter_vec(cmd.cpuid);
	let ignore_cpuid_filter = to_cpuid_filter_vec(cmd.ignore_cpuid);
	let mut output_vec = Vec::new();
	let mut codes = Vec::new();
	for (cpuid_str, cpuid, info) in all_cpuid {
		if !cmd.all && cpuid.len() == 1 && should_ignore_cpuid(cpuid[0]) {
			continue;
		}
		fn matches_cpuid(cpuid: &str, cpuid_pat: &str) -> bool {
			cpuid == cpuid_pat
		}
		if !cpuid_filter.is_empty() && !cpuid_filter.iter().any(|s| matches_cpuid(&cpuid_str, s)) {
			continue;
		}
		if !ignore_cpuid_filter.is_empty() && ignore_cpuid_filter.iter().any(|s| matches_cpuid(&cpuid_str, s)) {
			continue;
		}

		println!("{}", cpuid_str);
		if show_more_info {
			codes.clear();
			codes.extend(info.codes.values().copied());
			codes.sort_unstable_by_key(|info| (info.code.op_code().instruction_string(), info.code.op_code().op_code_string(), info.code));
			for info in codes.drain(..) {
				let opcode = info.code.op_code();
				output_vec.clear();
				if cmd.percent {
					output_vec.push(format!("{:.2}%", (info.count as f64) / (total_instrs as f64) * 100.));
				}
				if cmd.count {
					output_vec.push(format!("{}", info.count));
				}
				if cmd.opcode {
					output_vec.push(opcode.op_code_string().to_string());
				}
				if cmd.instr {
					output_vec.push(opcode.instruction_string().to_string());
				}
				println!("\t{}", output_vec.join(" | "));
			}
		}
	}

	Ok(())
}

const fn should_ignore_cpuid(cpuid: CpuidFeature) -> bool {
	matches!(
		cpuid,
		CpuidFeature::INTEL8086
			| CpuidFeature::INTEL8086_ONLY
			| CpuidFeature::INTEL186
			| CpuidFeature::INTEL286
			| CpuidFeature::INTEL286_ONLY
			| CpuidFeature::INTEL386
			| CpuidFeature::INTEL386_ONLY
			| CpuidFeature::INTEL386_A0_ONLY
			| CpuidFeature::INTEL486
			| CpuidFeature::INTEL486_A_ONLY
			| CpuidFeature::X64
			| CpuidFeature::CPUID
			| CpuidFeature::FPU
			| CpuidFeature::FPU287
			| CpuidFeature::FPU287XL_ONLY
			| CpuidFeature::FPU387
			| CpuidFeature::FPU387SL_ONLY
			| CpuidFeature::MULTIBYTENOP
			| CpuidFeature::PAUSE
			| CpuidFeature::RDPMC
			| CpuidFeature::SMM
	)
}
