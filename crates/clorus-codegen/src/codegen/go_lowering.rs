//! Normalization boundary for `go` bodies.
//!
//! Parking cannot be inferred by the runtime after LLVM has emitted an
//! ordinary blocking call.  Keep the source-level body and its direct parking
//! sites together here so the continuation emitter can replace the current
//! worker-pool implementation without changing reader or macro semantics.

use clorus_syntax::Expr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParkingOperation {
    Take,
    Put,
    Alts,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct GoBodyPlan {
    pub body: Expr,
    /// Direct forms in a normalized `do` body which need resumable lowering.
    /// Nested forms are deliberately not claimed as supported by Stage B yet.
    pub parking_sites: Vec<(usize, ParkingOperation)>,
}

impl GoBodyPlan {
    pub fn from_forms(forms: &[Expr]) -> Self {
        let body = match forms {
            [] => Expr::Nil,
            [expr] => expr.clone(),
            exprs => Expr::Do {
                exprs: exprs.to_vec(),
            },
        };
        let direct_forms = match &body {
            Expr::Do { exprs } => exprs.as_slice(),
            expr => std::slice::from_ref(expr),
        };
        let parking_sites = direct_forms
            .iter()
            .enumerate()
            .filter_map(|(index, expr)| match expr {
                Expr::Call { func, .. } if func == "<!" => Some((index, ParkingOperation::Take)),
                Expr::Call { func, .. } if func == ">!" => Some((index, ParkingOperation::Put)),
                Expr::Call { func, .. } if func == "alts!" => Some((index, ParkingOperation::Alts)),
                _ => None,
            })
            .collect();
        Self { body, parking_sites }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_forms_and_records_direct_parking_sites() {
        let plan = GoBodyPlan::from_forms(&[
            Expr::Call { func: ">!".into(), args: vec![Expr::Symbol("ch".into()), Expr::Long(1)] },
            Expr::Call { func: "<!".into(), args: vec![Expr::Symbol("ch".into())] },
        ]);
        assert!(matches!(plan.body, Expr::Do { .. }));
        assert_eq!(plan.parking_sites, vec![(0, ParkingOperation::Put), (1, ParkingOperation::Take)]);
    }
}
