//! The planner's tests, in three files by what they exercise. A test reaches the crate's own `plan`
//! and `plan_against` as `super::plan` and `super::plan_against`, because the helpers of the same
//! name in `test_support` shadow them.

use super::{plan, plan_against};

mod designation;
mod operands;
mod streams;
