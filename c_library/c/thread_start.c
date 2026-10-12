/*
 * Where a thread made by `pthread_create` starts its C code (milestone 836 (a C library, stage 2:
 * threads)); called from `c_library/src/pthread.rs`.
 *
 * `pthread_exit` must leave the thread's start routine from any depth and come back to the Rust
 * that spawned it, so that `std` can end the thread and free its stack. `setjmp` and `longjmp` are
 * how C does that, and Rust cannot call `setjmp` soundly (it returns twice, which Rust has no way
 * to declare), so this one function is C. `longjmp` lands here with `env`, and the value
 * `pthread_exit` stored in `*exit_value` is returned as the start routine's.
 */
#include <setjmp.h>

void *nife_pthread_trampoline(void *(*start)(void *), void *arg, unsigned long long *env,
                              void **exit_value)
{
    if (setjmp(env) != 0)
        return *exit_value;
    return start(arg);
}
