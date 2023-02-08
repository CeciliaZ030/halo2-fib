use super::*;

#[derive(Debug, Clone)]
pub struct FunctionConfig{
    selector: Selector,
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
    // if a==0 then b else c

    is_zero: IsZeroConfig,
    out: Column<Instance>,
}

pub struct FunctionChip<F: FieldExt>{
    config: FunctionConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FunctionChip<F> {
    pub fn construct(config: FunctionConfig) -> FunctionChip<F> {
        Self {
            config,
            _marker: PhantomData
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> FunctionConfig {
        let selector = meta.selector();
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let out = meta.instance_column();
        let is_zero = IsZeroChip::configure(
            meta,
            a,
            |meta| meta.query_selector(selector),
            |meta| meta.query_instance(out, Rotation::cur()) - meta.query_advice(b, Rotation::cur()), // if case
            |meta| meta.query_instance(out, Rotation::cur()) - meta.query_advice(c, Rotation::cur()), // else case
        );
        // 这种 button up的写法在 IsZeroChip::configure 中就录入constraint了
        // meta.create_gate("func", |meta| {});
        FunctionConfig {
            selector,
            a,
            b,
            c,
            is_zero,
            out
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: F, b: F, c: F, out: F
    ) -> Result<AssignedCell<F, F>, Error>{
        let is_zero_chip = IsZeroChip::construct(self.config.is_zero.clone());
        layouter.assign_region(
         || "func",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                is_zero_chip.assign(&mut region, 0, Value::known(a))?;
                region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;
                region.assign_advice(|| "c", self.config.c, 0, || Value::known(c))
            })
    }
}

#[derive(Default)]
pub struct FunctionCircuit<F> {
    pub a: F,
    pub b: F,
    pub c: F,
    pub out: F,
}

impl<F: FieldExt> Circuit<F> for FunctionCircuit<F> {
    type Config = FunctionConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FunctionChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FunctionChip::construct(config);
        chip.assign(layouter, self.a, self.b, self.c, self.out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_example3() {
        let circuit = FunctionCircuit {
            a: Fp::from(10),
            b: Fp::from(12),
            c: Fp::from(15),
            out: Fp::from(15)
        };

        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
        println!("Satisfied");
    }
}


