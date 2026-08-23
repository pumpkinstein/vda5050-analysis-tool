when building with `cargo rr`

 Compiling dioxus-router v0.7.10
error: rustc interrupted by SIGSEGV, printing backtrace

/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(+0x3c7cedc)[0x78091f47cedc]
/lib/x86_64-linux-gnu/libc.so.6(+0x45330)[0x78091b445330]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/libLLVM.so.22.1-rust-1.97.1-stable(_ZN4llvm24LowerExpectIntrinsicPass3runERNS_8FunctionERNS_15AnalysisManagerIS1_JEEE+0xcb)[0x7809195653e3]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/libLLVM.so.22.1-rust-1.97.1-stable(_ZN4llvm11PassManagerINS_8FunctionENS_15AnalysisManagerIS1_JEEEJEE3runERS1_RS3_+0x723)[0x78091940a123]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/libLLVM.so.22.1-rust-1.97.1-stable(_ZN4llvm27ModuleToFunctionPassAdaptor3runERNS_6ModuleERNS_15AnalysisManagerIS1_JEEE+0x325)[0x780919514d25]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/libLLVM.so.22.1-rust-1.97.1-stable(_ZN4llvm11PassManagerINS_6ModuleENS_15AnalysisManagerIS1_JEEEJEE3runERS1_RS3_+0x1ef)[0x780919565c2f]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(+0x6656b10)[0x780921e56b10]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(+0x664d2c0)[0x780921e4d2c0]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(_RNvXs1_CsojoDvyKp7v_18rustc_codegen_llvmNtB5_18LlvmCodegenBackendNtNtNtCs2NJjM5TEClf_17rustc_codegen_ssa6traits5write19WriteBackendMethods8optimize+0x33b)[0x780921e4ad7f]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(_RINvNtNtCs2AWtUsOyxgP_3std3sys9backtrace28___rust_begin_short_backtraceNCINvNtNtCs2NJjM5TEClf_17rustc_codegen_ssa4back5write10spawn_workNtCsojoDvyKp7v_18rustc_codegen_llvm18LlvmCodegenBackendE0uEB2c_+0x150)[0x780921f51d28]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(+0x675192b)[0x780921f5192b]
/home/moxx/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/librustc_driver-832cf6cfb1386559.so(+0x675556c)[0x780921f5556c]
/lib/x86_64-linux-gnu/libc.so.6(+0x9cb84)[0x78091b49cb84]
/lib/x86_64-linux-gnu/libc.so.6(+0x129d6c)[0x78091b529d6c]

note: we would appreciate a report at https://github.com/rust-lang/rust
help: you can increase rustc's stack size by setting RUST_MIN_STACK=16777216



Yes, file it. A stable rustc process terminating with SIGSEGV while compiling ordinary Rust code is report-worthy. The larger-stack workaround does not make it a routine nuisance; it provides useful diagnostic evidence.

A good issue title would be:

> rustc 1.97.1 SIGSEGV in LLVM LowerExpectIntrinsicPass compiling dioxus-router with target-cpu=native

Include:

- `rustc -vV`
- CPU model: AMD Ryzen 9 5950X
- Exact command and `.cargo/config.toml`
- `dioxus-router` version `0.7.10`
- The complete backtrace
- These two particularly useful observations:

  ```text
  Removing target-cpu=native avoids the crash.
  RUST_MIN_STACK=16777216 allows the complete HMI release build to succeed.
  ```

If practical, first confirm it using a fresh target directory without deleting your normal build cache:

```sh
CARGO_TARGET_DIR=/tmp/vda5050-rustc-repro \
cargo build --release -p hmi
```

Search the [Rust issue tracker](https://github.com/rust-lang/rust/issues) for `LowerExpectIntrinsicPass` first. If nothing matches, use the [compiler bug report form](https://github.com/rust-lang/rust/issues/new/choose).
