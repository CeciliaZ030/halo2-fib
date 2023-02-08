use std::marker::PhantomData;
use halo2_proofs::{plonk::*, circuit::*};
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::poly::Rotation;

#[derive(Clone)]
pub struct RangeCheckConfig<const RANGE: usize>{
    value: Column<Advice>,
    q: Selector
}

pub struct RangeCheckChip<F: FieldExt, const RANGE: usize>{
    config: RangeCheckConfig<RANGE>,
    _marker: PhantomData<F>
}

impl<F: FieldExt, const RANGE: usize> RangeCheckChip<F, RANGE> {
    fn construct(config: RangeCheckConfig<RANGE>) -> RangeCheckChip<F, RANGE> {
        Self { config, _marker: PhantomData }
    }

    fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> RangeCheckConfig<RANGE> {
        let q = meta.selector();
        meta.create_gate("RangeCheck", |meta| {
            let q = meta.query_selector(q);
            let v = meta.query_advice(value, Rotation::cur());
            let range_check_expr = |range: usize, value: Expression<F>| -> Expression<F> {
                let init = value.clone();
                (0..range).fold(init, |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                })
            };
            Constraints::with_selector(q, [("range_check", range_check_expr(RANGE, v))])
        });
        RangeCheckConfig {
            value,
            q
        }
    }
    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: F
    ) -> Result<(), Error> {
        layouter.assign_region(|| "Assign Value", |mut region| {
           self.config.q.enable(&mut region, 0);
            region.assign_advice(|| "value", self.config.value, 0, || Value::known(value))
        })
            .map(|cell| ())
    }
}

pub mod tests {
    use halo2_proofs::dev::{FailureLocation, MockProver, VerifyFailure};
    use halo2_proofs::pasta::Fp;
    use super::*;

    #[derive(Default)]
    pub struct RcCircuit<F, const RANGE: usize> {
        pub value: F,
    }

    impl<F: FieldExt, const RANGE: usize> Circuit<F> for RcCircuit<F, RANGE> {
        type Config = RangeCheckConfig<RANGE>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            RangeCheckChip::configure(meta, value)
        }

        fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
            let chip = RangeCheckChip::construct(config);
            chip.assign(layouter.namespace(|| "RangeCheckChip"), self.value)
        }
    }

    #[test]
    fn test_range_check() {
        let k = 4;
        const RANGE: usize = 9;

        //  in-range values
        for i in 0..RANGE {
            let circuit = RcCircuit::<Fp, RANGE> {
                value: Fp::from(i as u64)
            };
            let prover = MockProver::run(4, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

        // out-of-range
        let circuit = RcCircuit::<Fp, RANGE> { value: Fp::from(10 as u64) };
        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::ConstraintNotSatisfied {
                constraint: ((0, "RangeCheck").into(), 0, "range_check").into(),
                location: FailureLocation::InRegion {
                    region: (0, "Assign Value").into(),
                    offset: 0
                },
                cell_values: vec![(((Any::Advice, 0).into(), 0).into(), "0xa".to_string())]
            }])
        )
    }
}


