use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*};
use halo2_proofs::dev::MockProver;
use halo2_proofs::pasta::Fp;
use halo2_proofs::poly::Rotation;
use plotters::prelude::*;
use gadgets_lib::gadgets::*;

fn main() {
    let k = 4;
    const RANGE: usize = 9;
    let circuit = range_check::tests::RcCircuit::<Fp, RANGE> {
        value: Fp::from(7),
    };

    let root = BitMapBackend::new("range_check.png", (500, 1000)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.titled("Range Check Layout", ("sans-serif", 60)).unwrap();
    halo2_proofs::dev::CircuitLayout::default()
        .render(4, &circuit, &root)
        .unwrap();
}
