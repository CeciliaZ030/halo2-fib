use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*};
use halo2_proofs::dev::MockProver;
use halo2_proofs::pasta::Fp;
use halo2_proofs::poly::Rotation;

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig {
    pub advice: Column<Advice>,
    pub instance: Column<Instance>,
    pub selector: Selector,
}

struct FiboChip<F: FieldExt> {
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    // 给config，出chip
    fn construct(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData
        }
    }
    // 给搭建好的电路，出config
    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
        instance: Column<Instance>
    ) -> FiboConfig {
        let s = meta.selector();
        meta.enable_equality(instance);
        meta.enable_equality(advice);
        meta.create_gate("add", |meta|{
            let s = meta.query_selector(s);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            vec![s * (a + b - c)]
        });

        FiboConfig {
            advice,
            selector: s,
            instance
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Option<F>,
        b: Option<F>
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(
            ||"entire_table",
            |mut region| {
                self.config.selector.enable(&mut region, 0).unwrap();
                let mut a_cell = region
                    .assign_advice(|| "a", self.config.advice, 0, || a.ok_or(Error::Synthesis)).map(ACell)?;
                let mut b_cell = region
                    .assign_advice(|| "b", self.config.advice, 1, ||  b.ok_or(Error::Synthesis)).map(ACell)?;

                let (a_ret, b_ret) = (a_cell.clone(), b_cell.clone());

                for i in 2..10 {
                    let a = a_cell.0.value();
                    let b = b_cell.0.value();
                    let c = a.and_then(|a| b.map(|b| *a + *b));
                    let c_cell = region
                        .assign_advice(|| "c", self.config.advice, i, || c.ok_or(Error::Synthesis)).map(ACell)?;
                    a_cell = b_cell;
                    b_cell = c_cell;

                    if i == 9 { break; }
                    self.config.selector.enable(&mut region, i-1).unwrap();
                }
                Ok((a_ret, b_ret, b_cell))
            }
        )
    }

    pub fn expose_public(&self, mut layouter: impl Layouter<F>, cell: &ACell<F>, row: usize) {
        layouter.constrain_instance(cell.0.cell(), self.config.instance, row);
    }

}

#[derive(Default)]
struct MyCircuit<F>{
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();

        // 可以同一批列放在多个chip中，即自定义横向规划，reuse col
        let c1 = FiboChip::configure(meta, advice, instance);

        c1
    }
    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FiboChip::construct(config);
        let (prev_a, prev_b, last_c) = chip.assign(layouter.namespace(|| "Entire"), self.a, self.b)?;
        chip.expose_public(layouter.namespace(|| "private a"), &prev_a, 0);
        chip.expose_public(layouter.namespace(|| "private b"), &prev_b, 1);
        chip.expose_public(layouter.namespace(|| "out"), &last_c, 2);
        Ok(())
    }
}


fn main() {
    let k = 7;
    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);
    let circuit = MyCircuit {a: Some(a), b: Some(b)};

    let public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![public_input]).unwrap();
    prover.assert_satisfied();
}
