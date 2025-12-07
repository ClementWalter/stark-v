# stark-v runtime crate

This is the runtime every guest program links against to boot correctly on the
zkVM target.

Internally the crate installs a custom `_start` via inline assembly that sets
the stack pointer to `0x0020_0400`, then jumps into a Rust function
(`__starkv_start`) which calls the user entry point. Guests declare their entry
by writing `stark_v::entry!(main);`; the macro exports a symbol named
`starkv_entry` that wraps the user function. The runtime’s `_start` calls this
symbol after registers are initialized, so guests don’t need to write any
assembly. The crate also provides the panic handler used on the guest target (an
infinite loop for now) and a fallback handler for host builds so the workspace
compiles cleanly.
