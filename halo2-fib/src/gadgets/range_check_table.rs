use std::marker::PhantomData;
use halo2_proofs::{plonk::*, arithmetic::FieldExt};
use halo2_proofs::circuit::{Layouter, Value};
use halo2_proofs::poly::Rotation;

#[derive(Clone)]
pub struct RangeCheckTable<F, const NUM_BITS: usize> {
    pub table: TableColumn,
    _marker: PhantomData<F>
}

impl <F: FieldExt, const NUM_BITS: usize>  RangeCheckTable<F, NUM_BITS> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> RangeCheckTable<F, NUM_BITS>{
        RangeCheckTable{
            table: meta.lookup_table_column(),
            _marker: PhantomData
        }
    }
    pub fn load(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(|| "RangeCheckTable", |mut table| {
            for i in 0..(1<<NUM_BITS) {
                table.assign_cell(
                    || i.to_string(),
                    self.table,
                    i.clone(),
                    || Value::known(F::from(i as u64)))?;
            }
            Ok(())
        })
    }
}

#[derive(Clone)]
pub struct RangeCheckConfig<F, const NUM_BITS: usize>{
    value: Column<Advice>,
    table: RangeCheckTable<F, NUM_BITS>,
    q: Selector
}

pub struct RangeCheckChip<F: FieldExt, const NUM_BITS: usize>{
    config: RangeCheckConfig<F, NUM_BITS>,
    _marker: PhantomData<F>
}

impl<F: FieldExt, const NUM_BITS: usize> RangeCheckChip<F, NUM_BITS> {
    fn construct(config: RangeCheckConfig<F, NUM_BITS>) -> RangeCheckChip<F, NUM_BITS> {
        Self { config, _marker: PhantomData }
    }

    fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> RangeCheckConfig<F, NUM_BITS> {
        let q = meta.complex_selector();
        let value = meta.advice_column();
        let tableConfig = RangeCheckTable::configure(meta);

        meta.lookup(|meta| {
            let q = meta.query_selector(q);
            let value = meta.query_advice(value, Rotation::cur());
            vec![(q * value, tableConfig.table)]
        });
        RangeCheckConfig {
            value,
            table: tableConfig,
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
    use halo2_proofs::circuit::SimpleFloorPlanner;
    use halo2_proofs::dev::{FailureLocation, MockProver, VerifyFailure};
    use halo2_proofs::pasta::Fp;
    use super::*;

    #[derive(Default)]
    pub struct RcCircuit<F, const NUM_BITS: usize> {
        pub value: F,
    }

    impl<F: FieldExt, const NUM_BITS: usize> Circuit<F> for RcCircuit<F, NUM_BITS> {
        type Config = RangeCheckConfig<F, NUM_BITS>;
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
            chip.config.table.load(layouter.namespace(|| "RCTable"))?;
            chip.assign(layouter.namespace(|| "RangeCheckChip"), self.value)
        }
    }

    #[test]
    fn test_range_check() {
        let k = 12;
        const NUM_BITS: usize = 8;

        //  in-range values
        for i in 0..NUM_BITS {
            let circuit = RcCircuit::<Fp, NUM_BITS> {
                value: Fp::from(i as u64 * 7)
            };
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

        // out-of-range
        let circuit = RcCircuit::<Fp, NUM_BITS> { value: Fp::from(299 as u64) };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(
            prover.verify(),
            Err(vec![VerifyFailure::Lookup {
                lookup_index: 0,
                location: FailureLocation::InRegion {
                    region: (1, "Assign Value").into(),
                    offset: 0
                }
            }])
        )
    }
}
