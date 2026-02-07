//! Performance profiler for zkVM execution.
//!
//! Provides detailed performance analysis including:
//! - Cycle counting per instruction type
//! - Hot path identification
//! - Performance report generation
//! - Flame graph support

use crate::trace::Tracer;
use std::collections::HashMap;
use std::fmt;

/// Performance statistics for a single instruction type.
#[derive(Debug, Clone, Default)]
pub struct InstructionStats {
    /// Number of times this instruction was executed.
    pub count: u64,
    /// Percentage of total cycles.
    pub percentage: f64,
    /// Cumulative percentage (for sorting by frequency).
    pub cumulative_percentage: f64,
}

/// Hot spot in the execution trace - a PC location with high execution count.
#[derive(Debug, Clone)]
pub struct HotSpot {
    /// Program counter address.
    pub pc: u32,
    /// Number of times this PC was executed.
    pub hit_count: u64,
    /// Percentage of total cycles.
    pub percentage: f64,
}

/// Performance profile of a zkVM execution.
#[derive(Debug, Clone)]
pub struct ExecutionProfile {
    /// Total number of cycles.
    pub total_cycles: u64,
    /// Statistics per instruction type.
    pub instruction_stats: HashMap<String, InstructionStats>,
    /// Hot spots sorted by hit count (descending).
    pub hot_spots: Vec<HotSpot>,
    /// Total unique program counters visited.
    pub unique_pcs: usize,
}

impl ExecutionProfile {
    /// Generate a performance report as a formatted string.
    pub fn report(&self) -> String {
        let mut output = String::new();

        output.push_str("═══════════════════════════════════════════════════════════════\n");
        output.push_str("                    EXECUTION PROFILE REPORT                   \n");
        output.push_str("═══════════════════════════════════════════════════════════════\n\n");

        output.push_str(&format!("Total Cycles: {}\n", self.total_cycles));
        output.push_str(&format!("Unique PCs: {}\n\n", self.unique_pcs));

        output.push_str("───────────────────────────────────────────────────────────────\n");
        output.push_str("                 INSTRUCTION TYPE BREAKDOWN                     \n");
        output.push_str("───────────────────────────────────────────────────────────────\n\n");

        // Sort by count (descending)
        let mut sorted_stats: Vec<_> = self.instruction_stats.iter().collect();
        sorted_stats.sort_by(|a, b| b.1.count.cmp(&a.1.count));

        output.push_str(&format!(
            "{:<12} {:>12} {:>10} {:>10}\n",
            "Instruction", "Count", "Percent", "Cumulative"
        ));
        output.push_str(&format!("{}\n", "─".repeat(50)));

        for (name, stats) in sorted_stats.iter() {
            output.push_str(&format!(
                "{:<12} {:>12} {:>9.2}% {:>9.2}%\n",
                name, stats.count, stats.percentage, stats.cumulative_percentage
            ));
        }

        output.push_str("\n───────────────────────────────────────────────────────────────\n");
        output.push_str("                     TOP HOT SPOTS (Top 20)                    \n");
        output.push_str("───────────────────────────────────────────────────────────────\n\n");

        output.push_str(&format!(
            "{:<12} {:>12} {:>10}\n",
            "PC", "Hit Count", "Percent"
        ));
        output.push_str(&format!("{}\n", "─".repeat(40)));

        for hot_spot in self.hot_spots.iter().take(20) {
            output.push_str(&format!(
                "0x{:<10x} {:>12} {:>9.2}%\n",
                hot_spot.pc, hot_spot.hit_count, hot_spot.percentage
            ));
        }

        output.push_str("\n═══════════════════════════════════════════════════════════════\n");

        output
    }

    /// Export profile data in a format suitable for flame graph generation.
    /// Returns lines in the "folded stack" format: "frame1;frame2;frame3 count"
    pub fn to_flame_graph_folded(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // For each hot spot, create a line with PC as the stack frame
        for hot_spot in &self.hot_spots {
            lines.push(format!("pc_0x{:08x} {}", hot_spot.pc, hot_spot.hit_count));
        }

        lines
    }
}

impl fmt::Display for ExecutionProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.report())
    }
}

/// Profiler for analyzing zkVM execution performance.
pub struct Profiler {
    /// Instruction counts by opcode.
    instruction_counts: HashMap<String, u64>,
    /// PC hit counts.
    pc_counts: HashMap<u32, u64>,
    /// Total cycles.
    total_cycles: u64,
}

impl Profiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            instruction_counts: HashMap::new(),
            pc_counts: HashMap::new(),
            total_cycles: 0,
        }
    }

    /// Profile a tracer to collect execution statistics.
    ///
    /// This analyzes the trace tables to count instruction types and identify hot spots.
    pub fn profile_tracer(&mut self, tracer: &Tracer) {
        // Total cycles is the total number of trace entries across all tables
        self.total_cycles = tracer.total_traces() as u64;

        // Count instructions by type based on trace table entries
        self.count_instruction_type("base_alu_reg", tracer.base_alu_reg.len() as u64);
        self.count_instruction_type("base_alu_imm", tracer.base_alu_imm.len() as u64);
        self.count_instruction_type("shifts_reg", tracer.shifts_reg.len() as u64);
        self.count_instruction_type("shifts_imm", tracer.shifts_imm.len() as u64);
        self.count_instruction_type("lt_reg", tracer.lt_reg.len() as u64);
        self.count_instruction_type("lt_imm", tracer.lt_imm.len() as u64);
        self.count_instruction_type("branch_eq", tracer.branch_eq.len() as u64);
        self.count_instruction_type("branch_lt", tracer.branch_lt.len() as u64);
        self.count_instruction_type("lui", tracer.lui.len() as u64);
        self.count_instruction_type("auipc", tracer.auipc.len() as u64);
        self.count_instruction_type("jalr", tracer.jalr.len() as u64);
        self.count_instruction_type("jal", tracer.jal.len() as u64);
        self.count_instruction_type("load_store", tracer.load_store.len() as u64);
        self.count_instruction_type("mul", tracer.mul.len() as u64);
        self.count_instruction_type("mulh", tracer.mulh.len() as u64);
        self.count_instruction_type("div", tracer.div.len() as u64);

        // Collect PC counts from trace tables
        self.collect_pc_counts(tracer);
    }

    /// Count an instruction type.
    fn count_instruction_type(&mut self, name: &str, count: u64) {
        if count > 0 {
            *self.instruction_counts.entry(name.to_string()).or_insert(0) += count;
        }
    }

    /// Collect PC counts from trace tables.
    fn collect_pc_counts(&mut self, tracer: &Tracer) {
        // Collect from all tables that have PC columns
        for i in 0..tracer.base_alu_reg.len() {
            let pc = tracer.base_alu_reg.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.base_alu_imm.len() {
            let pc = tracer.base_alu_imm.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.shifts_reg.len() {
            let pc = tracer.shifts_reg.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.shifts_imm.len() {
            let pc = tracer.shifts_imm.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.lt_reg.len() {
            let pc = tracer.lt_reg.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.lt_imm.len() {
            let pc = tracer.lt_imm.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.branch_eq.len() {
            let pc = tracer.branch_eq.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.branch_lt.len() {
            let pc = tracer.branch_lt.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.lui.len() {
            let pc = tracer.lui.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.auipc.len() {
            let pc = tracer.auipc.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.jalr.len() {
            let pc = tracer.jalr.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.jal.len() {
            let pc = tracer.jal.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.load_store.len() {
            let pc = tracer.load_store.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.mul.len() {
            let pc = tracer.mul.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.mulh.len() {
            let pc = tracer.mulh.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }

        for i in 0..tracer.div.len() {
            let pc = tracer.div.pc[i];
            *self.pc_counts.entry(pc).or_insert(0) += 1;
        }
    }

    /// Generate an execution profile from the collected statistics.
    pub fn generate_profile(&self) -> ExecutionProfile {
        let mut instruction_stats = HashMap::new();
        let mut cumulative = 0.0;

        // Sort instructions by count for cumulative percentage
        let mut sorted_counts: Vec<_> = self.instruction_counts.iter().collect();
        sorted_counts.sort_by(|a, b| b.1.cmp(a.1));

        for (name, &count) in sorted_counts {
            let percentage = if self.total_cycles > 0 {
                (count as f64 / self.total_cycles as f64) * 100.0
            } else {
                0.0
            };
            cumulative += percentage;

            instruction_stats.insert(
                name.clone(),
                InstructionStats {
                    count,
                    percentage,
                    cumulative_percentage: cumulative,
                },
            );
        }

        // Generate hot spots
        let mut hot_spots: Vec<_> = self
            .pc_counts
            .iter()
            .map(|(&pc, &hit_count)| {
                let percentage = if self.total_cycles > 0 {
                    (hit_count as f64 / self.total_cycles as f64) * 100.0
                } else {
                    0.0
                };
                HotSpot {
                    pc,
                    hit_count,
                    percentage,
                }
            })
            .collect();

        // Sort by hit count descending
        hot_spots.sort_by(|a, b| b.hit_count.cmp(&a.hit_count));

        ExecutionProfile {
            total_cycles: self.total_cycles,
            instruction_stats,
            hot_spots,
            unique_pcs: self.pc_counts.len(),
        }
    }

    /// Profile a tracer and immediately generate a report.
    ///
    /// This is a convenience method that combines `profile_tracer` and `generate_profile`.
    pub fn profile_and_report(tracer: &Tracer) -> ExecutionProfile {
        let mut profiler = Self::new();
        profiler.profile_tracer(tracer);
        profiler.generate_profile()
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Access;

    #[test]
    fn test_profiler_empty_tracer() {
        let tracer = Tracer::default();
        let profile = Profiler::profile_and_report(&tracer);

        assert_eq!(profile.total_cycles, 0);
        assert_eq!(profile.unique_pcs, 0);
        assert!(profile.instruction_stats.is_empty());
        assert!(profile.hot_spots.is_empty());
    }

    #[test]
    fn test_profiler_with_traces() {
        let mut tracer = Tracer::default();

        // Add some traces
        let access = Access::default();
        tracer
            .base_alu_reg
            .push(0, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_reg
            .push(1, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_imm
            .push(2, 0x1004, access, access, 0, 0, 0, 1, 0, 0, 0);
        tracer.lui.push(3, 0x1008, access, 0, 0, 0);

        let profile = Profiler::profile_and_report(&tracer);

        assert_eq!(profile.total_cycles, 4);
        assert_eq!(profile.unique_pcs, 3);

        // Check instruction stats
        assert_eq!(
            profile.instruction_stats.get("base_alu_reg").unwrap().count,
            2
        );
        assert_eq!(
            profile.instruction_stats.get("base_alu_imm").unwrap().count,
            1
        );
        assert_eq!(profile.instruction_stats.get("lui").unwrap().count, 1);

        // Check hot spots
        assert!(!profile.hot_spots.is_empty());
        let hottest = &profile.hot_spots[0];
        assert_eq!(hottest.pc, 0x1000);
        assert_eq!(hottest.hit_count, 2);
        assert_eq!(hottest.percentage, 50.0);
    }

    #[test]
    fn test_profile_report_format() {
        let mut tracer = Tracer::default();

        let access = Access::default();
        tracer
            .base_alu_reg
            .push(0, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_imm
            .push(1, 0x1004, access, access, 0, 0, 0, 1, 0, 0, 0);

        let profile = Profiler::profile_and_report(&tracer);
        let report = profile.report();

        assert!(report.contains("EXECUTION PROFILE REPORT"));
        assert!(report.contains("Total Cycles: 2"));
        assert!(report.contains("INSTRUCTION TYPE BREAKDOWN"));
        assert!(report.contains("TOP HOT SPOTS"));
    }

    #[test]
    fn test_flame_graph_export() {
        let mut tracer = Tracer::default();

        let access = Access::default();
        tracer
            .base_alu_reg
            .push(0, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_reg
            .push(1, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_imm
            .push(2, 0x1004, access, access, 0, 0, 0, 1, 0, 0, 0);

        let profile = Profiler::profile_and_report(&tracer);
        let folded = profile.to_flame_graph_folded();

        assert_eq!(folded.len(), 2);
        assert!(folded[0].contains("pc_0x00001000 2"));
        assert!(folded[1].contains("pc_0x00001004 1"));
    }

    #[test]
    fn test_percentages_sum_to_100() {
        let mut tracer = Tracer::default();

        let access = Access::default();
        tracer
            .base_alu_reg
            .push(0, 0x1000, access, access, access, 1, 0, 0, 0, 0);
        tracer
            .base_alu_imm
            .push(1, 0x1004, access, access, 0, 0, 0, 1, 0, 0, 0);
        tracer.lui.push(2, 0x1008, access, 0, 0, 0);
        tracer.jal.push(3, 0x100c, access, 0);

        let profile = Profiler::profile_and_report(&tracer);

        let total_percentage: f64 = profile
            .instruction_stats
            .values()
            .map(|stats| stats.percentage)
            .sum();

        assert!((total_percentage - 100.0).abs() < 0.01);
    }
}
