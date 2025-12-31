import Lake
open System Lake DSL

package test

@[test_driver, lint_driver, bench_driver]
lean_exe driver
