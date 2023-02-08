use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*};
use halo2_proofs::dev::MockProver;
use halo2_proofs::pasta::Fp;
use halo2_proofs::poly::Rotation;

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct FiboConfig {
    pub advice: [Column<Advice>; 3],
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
        advices: [Column<Advice>; 3],
        instance: Column<Instance>
    ) -> FiboConfig {
        let s = meta.selector();
        meta.enable_equality(instance);
        for advice in advices {
            meta.enable_equality(advice);
        }
        meta.create_gate("add", |meta|{
            let s = meta.query_selector(s);
            let a = meta.query_advice(advices[0], Rotation::cur());
            let b = meta.query_advice(advices[1], Rotation::cur());
            let c = meta.query_advice(advices[2], Rotation::cur());
            vec![s * (a + b - c)]
        });

        FiboConfig {
            advice: advices,
            selector: s,
            instance
        }
    }

    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
        a: Option<F>,
        b: Option<F>
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(
            ||"_first_row",
            |mut region| {
                self.config.selector.enable(&mut region, 0).unwrap();
                let c_val = a.and_then(|a| b.map(|b| a + b));

                // region.assign_advice 赋值并返回新的cell指针
                // {val=新值 cell=(region#, offset, col#)}

                let a_cell = region
                    .assign_advice(|| "a", self.config.advice[0], 0, || Value::known(a.unwrap()))
                    .map(ACell)?;
                let b_cell = region
                    .assign_advice(|| "b", self.config.advice[1], 0, ||  Value::known(b.unwrap()))
                    .map(ACell)?;
                let c_cell = region
                    .assign_advice(|| "c", self.config.advice[2], 0, || Value::known(c_val.unwrap()))
                    .map(ACell)?;
                Ok((a_cell, b_cell, c_cell))
            }
        )
    }

    fn assign_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev_b: &ACell<F>,
        prev_c: &ACell<F>
    ) -> Result<(ACell<F>, ACell<F>), Error>
    {
        layouter.assign_region(
            || "_next_row",
            | mut region| {
                let c_val = prev_b.0.value().and_then(|b| prev_c.0.value().map(|c| *b + *c));
                self.config.selector.enable(&mut region, 0)?;

                // AssignedCell.copy_advice 将自己的值赋给局部region的相对位置cell
                // 并enable equality，如 (region#=0, offset=0, col#=1) == (region#=1, offset=0, col#=1) 此处为permutation
                // 返回新的被赋值的指针  (region#=1, offset=0, col#=1)

                let a = prev_b.0.copy_advice(|| "a", &mut region, self.config.advice[0], 0).map(ACell)?;
                let b = prev_c.0.copy_advice(|| "b", &mut region, self.config.advice[1], 0).map(ACell)?;
                let c = region
                    .assign_advice(|| "c", self.config.advice[2], 0, || c_val)
                    .map(ACell)?;

                let bc= b.0.cell();
                Ok((b, c))
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
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let instance = meta.instance_column();

        // 可以同一批列放在多个chip中，即自定义横向规划，reuse col
        let c1 = FiboChip::configure(meta, [col_a, col_b, col_c], instance);

        c1
    }
    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FiboChip::construct(config);
        let (prev_a, mut prev_b, mut prev_c) = chip.assign_first_row(
            layouter.namespace(|| "FirstRow"),
            self.a.clone(),
            self.b.clone()
        )?;
        chip.expose_public(layouter.namespace(|| "private a"), &prev_a, 0);
        chip.expose_public(layouter.namespace(|| "private b"), &prev_b, 1);

        for _ in 1..8 {
            let (b, c) = chip.assign_row(
                layouter.namespace(|| "NextRow"),
                &prev_b,
                &prev_c
            )?;
            prev_b = b;
            prev_c = c;
        }
        chip.expose_public(layouter.namespace(|| "out"), &prev_c, 2);


        Ok(())
    }
}


fn main() {
    let k = 4;
    let a = Fp::from(1);
    let b = Fp::from(1);
    let out = Fp::from(55);
    let circuit = MyCircuit {a: Some(a), b: Some(b)};

    let public_input = vec![a, b, out];

    let prover = MockProver::run(k, &circuit, vec![public_input]).unwrap();

    use plotters::prelude::*;
    let root = BitMapBackend::new("fib-1-layout.png", (500, 1000)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.titled("Fib 1 Layout", ("sans-serif", 60)).unwrap();
    halo2_proofs::dev::CircuitLayout::default()
        .show_equality_constraints(true)
        .render(4, &circuit, &root)
        .unwrap();

}
