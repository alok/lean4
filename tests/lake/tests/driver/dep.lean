import Lake
open Lake DSL

package test where
  testDriver := "dep/driver"
  lintDriver := "dep/driver"
  benchDriver := "dep/driver"

require dep from "dep"
