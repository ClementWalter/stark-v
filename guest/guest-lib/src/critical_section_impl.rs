use critical_section::RawRestoreState;

// Single-threaded zkVM guest execution has no interrupts or concurrency.
struct ZkvmCriticalSection;

critical_section::set_impl!(ZkvmCriticalSection);

unsafe impl critical_section::Impl for ZkvmCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        ()
    }

    unsafe fn release(_: RawRestoreState) {}
}
