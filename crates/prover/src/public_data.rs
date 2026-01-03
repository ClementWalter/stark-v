//! Public data for RV32IM proofs.
//!
//! Captures public execution state and provides LogUp compensation entries.

use num_traits::Zero;
use stwo::core::channel::Channel;
use stwo::core::fields::FieldExpOps;
use stwo::core::fields::m31::{M31, P as M31_P};
use stwo::core::fields::qm31::QM31;
use stwo_constraint_framework::Relation;

use crate::relations::Relations;

/// Public data required to verify an RV32IM proof.
#[derive(Clone, Debug)]
pub struct PublicData {
    /// Entry PC at start of execution.
    pub initial_pc: u32,
    /// PC at end of execution (next instruction after last).
    pub final_pc: u32,
    /// Total number of executed cycles (last instruction clk).
    pub clock: u32,
    /// Register values at start (x0..x31).
    pub initial_regs: [u32; 32],
    /// Register values at end (x0..x31).
    pub final_regs: [u32; 32],
    /// Last access clock per register (0 if never accessed).
    pub reg_last_clk: [u32; 32],
    /// Program tree root (if program table is non-empty).
    pub program_root: Option<u32>,
    /// RW initial memory tree root (if memory table is non-empty).
    pub initial_rw_root: Option<u32>,
    /// RW final memory tree root (if memory table is non-empty).
    pub final_rw_root: Option<u32>,
}

impl PublicData {
    pub fn new(run_result: &runner::RunResult) -> Self {
        let tracer = &run_result.tracer;

        let program_root = tracer.program.root.first().copied();

        let mut initial_rw_root = None;
        let mut final_rw_root = None;
        for (root, mult) in tracer
            .memory
            .root
            .iter()
            .zip(tracer.memory.multiplicity.iter())
        {
            if *mult == 1 && initial_rw_root.is_none() {
                initial_rw_root = Some(*root);
            }
            if *mult == M31_P - 1 && final_rw_root.is_none() {
                final_rw_root = Some(*root);
            }
            if initial_rw_root.is_some() && final_rw_root.is_some() {
                break;
            }
        }

        let clock = u32::try_from(run_result.cycles).expect("cycles overflow u32");

        Self {
            initial_pc: run_result.initial_pc,
            final_pc: run_result.final_pc,
            clock,
            initial_regs: run_result.initial_regs,
            final_regs: run_result.final_regs,
            reg_last_clk: tracer.reg_clk,
            program_root,
            initial_rw_root,
            final_rw_root,
        }
    }

    /// Mix public data into the channel transcript.
    pub fn mix_into(&self, channel: &mut impl Channel) {
        channel.mix_u32s(&[self.initial_pc, self.final_pc, self.clock]);
        channel.mix_u32s(&self.initial_regs);
        channel.mix_u32s(&self.final_regs);
        channel.mix_u32s(&self.reg_last_clk);

        let root_flags = [
            self.program_root.is_some() as u32,
            self.initial_rw_root.is_some() as u32,
            self.final_rw_root.is_some() as u32,
        ];
        channel.mix_u32s(&root_flags);
        let roots = [
            self.program_root.unwrap_or(0),
            self.initial_rw_root.unwrap_or(0),
            self.final_rw_root.unwrap_or(0),
        ];
        channel.mix_u32s(&roots);
    }

    /// LogUp sum contribution from public data.
    pub fn logup_sum(&self, relations: &Relations) -> QM31 {
        let mut values_to_inverse: Vec<QM31> = Vec::new();

        // Registers state: emit initial (pc, clk=1), consume final (pc, clk=clock+1).
        if self.clock > 0 {
            let initial_clk = M31::from(1u32);
            let final_clk = M31::from(
                self.clock
                    .checked_add(1)
                    .expect("clock overflow when computing final clk"),
            );
            values_to_inverse.push(
                relations
                    .registers_state
                    .combine(&[M31::from(self.initial_pc), initial_clk]),
            );
            let final_state: QM31 = relations
                .registers_state
                .combine(&[M31::from(self.final_pc), final_clk]);
            values_to_inverse.push(-final_state);
        }

        // Merkle roots: emit each tree root once.
        for root in [self.program_root, self.initial_rw_root, self.final_rw_root]
            .into_iter()
            .flatten()
        {
            values_to_inverse.push(relations.merkle.combine(&[
                M31::zero(),
                M31::zero(),
                M31::from(root),
                M31::from(root),
            ]));
        }

        // Register memory access: emit initial state (clk=0), consume final state (clk=last).
        let reg_as = M31::zero();
        for (idx, &last_clk) in self.reg_last_clk.iter().enumerate() {
            if last_clk == 0 {
                continue;
            }
            let addr = M31::from(idx as u32);
            let init_bytes = self.initial_regs[idx].to_le_bytes();
            values_to_inverse.push(relations.memory_access.combine(&[
                reg_as,
                addr,
                M31::zero(),
                M31::from(init_bytes[0] as u32),
                M31::from(init_bytes[1] as u32),
                M31::from(init_bytes[2] as u32),
                M31::from(init_bytes[3] as u32),
            ]));

            let final_bytes = self.final_regs[idx].to_le_bytes();
            let final_access: QM31 = relations.memory_access.combine(&[
                reg_as,
                addr,
                M31::from(last_clk),
                M31::from(final_bytes[0] as u32),
                M31::from(final_bytes[1] as u32),
                M31::from(final_bytes[2] as u32),
                M31::from(final_bytes[3] as u32),
            ]);
            values_to_inverse.push(-final_access);
        }

        if values_to_inverse.is_empty() {
            return QM31::zero();
        }

        let inverses = QM31::batch_inverse(&values_to_inverse);
        inverses.iter().sum()
    }
}
