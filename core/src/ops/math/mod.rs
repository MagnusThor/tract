pub mod mat_mat_mul;
pub mod mat_mul;

pub use self::mat_mul::MatMul;
use crate::internal::*;
use num_traits::{Float, Zero};

use super::binary::*;

bin_to_super_type!(add, Add,
        flip:commute,
        validation: Validation::Rounding,
     [f32, i8, i16, i32, i64, u8, u16, f16, f64, TDim] => |c, a, b| *c = a.clone() + b);
bin_to_super_type!(sub, Sub, flip:flip_sub,
     [f32, i8, i16, i32, i64, u8, u16, f16, f64, TDim] => |c, a, b| *c = a.clone() - b);
#[inline]
bin_to_super_type!(mul, Mul,
        cost: |dt| tvec!((Cost::FMA(dt), 1)),
        declutter_unary: declutter_mul_as_shift,
        flip: commute,
     [f32, i8, i16, i32, i64, u8, u16, f16, f64, TDim] => |c, a, b| *c = a.clone() * b);
bin_to_super_type!(div, Div,
        cost: |dt| tvec!((Cost::Div(dt), 1)),
     [f32, i8, i16, i32, i64, u8, u16, f16, f64, TDim] => |c, a, b| *c = a.clone() / b);
bin_to_super_type!(rem, Rem,
     [f32, i8, i16, i32, i64, u8, u16, f16, f64, TDim] => |c, a, b| *c = a.clone() % b);
bin_to_super_type!(min, Min, flip:commute,
     [f32, f64] => |c,a,b| *c = a.min(*b),
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *a.min(b));
bin_to_super_type!(max, Max, flip:commute,
     [f32, f64] => |c,a,b| *c = a.max(*b),
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *a.max(b));
bin_to_super_type!(pow, Pow,
     [f32, f64] => |c,a,b| *c = a.powf(*b));

bin_to_super_type!(shift_left, ShiftLeft,
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *a << *b);
bin_to_super_type!(shift_right, ShiftRight,
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *a >> *b);
bin_to_super_type!(flipped_shift_left, FlippedShiftLeft,
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *b << *a);
bin_to_super_type!(flipped_shift_right, FlippedShiftRight,
     [i8, i16, i32, i64, u8, u16] => |c, a, b| *c = *b >> *a);

fn flip_sub(_op: &dyn BinMiniOp, t: &Arc<Tensor>) -> Option<UnaryOp> {
    let mut t = t.clone().into_tensor();
    fn negate<T: Datum + std::ops::Neg<Output = T>>(t: &mut Tensor) {
        t.as_slice_mut::<T>().unwrap().iter_mut().for_each(|p| *p = -p.clone());
    }
    (|t: &mut Tensor| -> TractResult<()> {
        dispatch_signed!(negate(t.datum_type())(t));
        Ok(())
    })(&mut t)
    .unwrap();
    Some(UnaryOp::new(Box::new(Add), Arc::new(t)))
}

fn declutter_mul_as_shift(
    _op: &Mul,
    model: &TypedModel,
    node: &TypedNode,
    a: &Arc<Tensor>,
) -> TractResult<Option<TypedModelPatch>> {
    let input = model.node_input_facts(node.id)?[0];
    if a.len() > 0
        && a.is_uniform()?
        && a.datum_type().is_integer()
        && input.datum_type.is_integer()
    {
        let arg = a.cast_to::<i64>()?;
        let arg = arg.as_slice::<i64>()?[0];
        if arg.abs().count_ones() == 1 {
            let shift = (63 - arg.abs().leading_zeros()) as i32;
            let shift = tensor0(shift).cast_to_dt(a.datum_type())?.into_owned();
            let mini_op: Box<dyn BinMiniOp> =
                if arg > 0 { Box::new(FlippedShiftLeft) } else { Box::new(FlippedShiftRight) };
            return Ok(Some(TypedModelPatch::replace_single_op(
                model,
                node,
                &node.inputs,
                UnaryOp { a: shift.into_arc_tensor(), mini_op },
            )?));
        }
    }
    Ok(None)
}

element_wise!(abs, Abs, [f16, f32, i32] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.abs());
    Ok(())
});

element_wise!(exp, Exp, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.exp());
    Ok(())
};
    validation: Validation::Rounding
);

element_wise!(ln, Ln, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.ln());
    Ok(())
};
    validation: Validation::Rounding
);

element_wise!(sqrt, Sqrt, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.sqrt());
    Ok(())
};
    validation: Validation::Rounding
);

element_wise!(recip, Recip, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.recip());
    Ok(())
};
    validation: Validation::Rounding
);

element_wise!(rsqrt, Rsqrt, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.sqrt().recip());
    Ok(())
};
    validation: Validation::Rounding
);

element_wise!(ceil, Ceil, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.ceil());
    Ok(())
});

element_wise!(floor, Floor, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.floor());
    Ok(())
});

element_wise!(scalar_min_max, ScalarMinMax { min: Tensor, max: Tensor },
   [f32, f64] => |m, xs| {
        let max = m.max.cast_to_scalar()?;
        let min = m.min.cast_to_scalar()?;
        xs.iter_mut().for_each(|x| { *x = x.max(max).min(min) });
        Ok(())
});

element_wise!(scalar_min, ScalarMin { min: Tensor },
   [f32, f64] => |m, xs| {
        let min = m.min.cast_to_scalar()?;
        xs.iter_mut().for_each(|x| *x = x.min(min));
        Ok(())
});

element_wise!(scalar_max, ScalarMax { max: Tensor },
   [f32, f64] => |m, xs| {
        let max = m.max.cast_to_scalar()?;
        xs.iter_mut().for_each(|x| *x = x.max(max));
        Ok(())
});

element_wise!(cos, Cos, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.cos());
    Ok(())
});

element_wise!(sin, Sin, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.sin());
    Ok(())
});

element_wise!(tan, Tan, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.tan());
    Ok(())
});

element_wise!(acos, Acos, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.acos());
    Ok(())
});

element_wise!(asin, Asin, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.asin());
    Ok(())
});

element_wise!(atan, Atan, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.atan());
    Ok(())
});

element_wise!(cosh, Cosh, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.cosh());
    Ok(())
});

element_wise!(sinh, Sinh, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = x.sinh());
    Ok(())
});

element_wise!(tanh, Tanh,
   [f32] => |_, xs| { (tract_linalg::ops().stanh)().run(xs); Ok(()) },
   [f16, f64] => |_, xs| { xs.iter_mut().for_each(|x| *x = x.tanh()); Ok(()) };
   cost: |dt| {tvec!((Cost::FMA(dt), 11), (Cost::Div(dt), 1))}
);

element_wise!(acosh, Acosh, [f16, f32, f64] => |_, xs| { xs.iter_mut().for_each(|x| *x = x.acosh()); Ok(()) });
element_wise!(asinh, Asinh, [f16, f32, f64] => |_, xs| { xs.iter_mut().for_each(|x| *x = x.asinh()); Ok(()) });
element_wise!(atanh, Atanh, [f16, f32, f64] => |_, xs| { xs.iter_mut().for_each(|x| *x = x.atanh()); Ok(()) });

element_wise!(neg, Neg, [i8, i16, i32, i64, f16, f32, f64, TDim] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = -x.clone());
    Ok(())
});

element_wise!(sign, Sign, [f16, f32, f64] => |_, xs| {
    xs.iter_mut().for_each(|x| *x = if x.is_zero() { *x } else { x.signum() });
    Ok(())
});

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr2;

    #[test]
    fn mul() {
        let a = arr2(&[[1., 2.], [3., 4.]]);
        let b = arr2(&[[1., 0.], [0., 0.]]);
        assert_eq!(a * b, arr2(&[[1., 0.], [0., 0.]]));
    }

    #[test]
    fn dot() {
        let a = arr2(&[[1., 2.], [3., 4.]]);
        let b = arr2(&[[1., 0.], [0., 0.]]);
        assert_eq!(a.dot(&b), arr2(&[[1., 0.], [3., 0.]]));
    }

    #[test]
    fn mul_as_shift() -> TractResult<()> {
        let mut model = TypedModel::default();
        let x =
            model.add_source("a", TypedFact::dt_shape(i32::datum_type(), [2usize, 2].as_ref())?)?;
        let y = model.wire_node("c", mul::unary(rctensor0(4)), [x].as_ref())?[0];
        model.set_output_outlets(&[y])?;
        let result = SimplePlan::new(&model)?.run(tvec!(tensor2(&[[1, 2], [3, 4]])))?;
        assert_eq!(result[0], rctensor2(&[[4, 8], [12, 16]]));
        let decluttered = model.declutter()?;
        dbg!(&decluttered);
        let result = SimplePlan::new(&decluttered)?.run(tvec!(tensor2(&[[1, 2], [3, 4]])))?;
        assert_eq!(result[0], rctensor2(&[[4, 8], [12, 16]]));
        let op = decluttered.node_op(1).downcast_ref::<UnaryOp>().unwrap();
        assert!(op.mini_op.downcast_ref::<FlippedShiftLeft>().is_some());
        Ok(())
    }
}
