use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*};
use halo2_proofs::dev::MockProver;
use halo2_proofs::pasta::Fp;
use halo2_proofs::poly::Rotation;
use plotters::prelude::*;
use gadgets_lib::gadgets::*;

fn main() {
    let k = 4;
    let circuit = is_zero2::FunctionCircuit {
        a: Fp::from(10),
        b: Fp::from(12),
        c: Fp::from(15),
        //out: Fp::from(15)
    };

    let root = BitMapBackend::new("example-3-layout.png", (500, 1000)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.titled("Example 33 Layout", ("sans-serif", 60)).unwrap();
    halo2_proofs::dev::CircuitLayout::default()
        .render(4, &circuit, &root)
        .unwrap();
}
