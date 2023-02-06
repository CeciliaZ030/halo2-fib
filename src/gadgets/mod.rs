use std::marker::PhantomData;
use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
pub mod example3;
pub mod is_zero2;

#[derive(Clone, Debug)]
pub struct IsZeroConfig {
    pub value_inv: Column<Advice>,
}

//                                  is_zero_expr
// valid | value |  value_inv |  1 - value * value_inv | a * b * (1 - value * value_inv)
// ------+-------+------------+------------------------+-------------------------------
//  yes  |   x   |    1/x     |         0              |  0
//  yes  |   0   |    0       |         1              |  q
//

pub struct IsZeroChip<F: FieldExt>{
    pub config: IsZeroConfig,
    _marker: PhantomData<F>,

}

impl<F: FieldExt> IsZeroChip<F> {
    pub fn construct(config: IsZeroConfig) -> IsZeroChip<F> {
        Self{
            config,
            _marker: PhantomData
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>, // condition

        up_s: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        up_if: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        up_else: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> IsZeroConfig {
        // 该需要一个新col
        let value_inv = meta.advice_column();
        // 新的 expr
        let mut is_zero_expr = Expression::Constant(F::zero());

        meta.create_gate("is_zero", |meta| {
            let value = meta.query_advice(value, Rotation::cur());
            let value_inv = meta.query_advice(value_inv, Rotation::cur());
            let up_s = up_s(meta);
            let up_if = up_if(meta);
            let up_else = up_else(meta);

            // 1 - x * 1/x 除法只能在assign的时候算，填入一个新col
            // Expression 实际上是 poly ring operation 没有除法
            is_zero_expr = Expression::Constant(F::one()) - value.clone() * value_inv;
            vec![
                up_s.clone() * up_if * is_zero_expr.clone(),
                up_s.clone() * up_else * (Expression::Constant(F::one()) - is_zero_expr.clone())
            ]
        });
        IsZeroConfig {
            value_inv,
        }
    }

    pub fn assign(&self, region: &mut Region<'_, F>, offset: usize, value: Value<F>) -> Result<(), Error>{
        let value_inv = value.map(|value| value.invert().unwrap_or(F::zero()));
        // 用 region 而不是 layouter
        // 因为 isZero 是外层电路板的一个region，使用者不对 isZero 实例化电路
        region.assign_advice(
            || "value inverse", self.config.value_inv, offset, || value_inv
        )?;
        Ok(())
    }

}


