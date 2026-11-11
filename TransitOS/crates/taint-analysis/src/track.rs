use crate::taint::{TaintStatus, TaintedValue};

pub fn propagate<T, U>(input: &TaintedValue<T>, output: U) -> TaintedValue<U> {
    TaintedValue {
        value: output,
        taint: input.taint,
        sources: input.sources.clone(),
    }
}

pub fn combine<T, U, V>(a: &TaintedValue<T>, b: &TaintedValue<U>, output: V) -> TaintedValue<V> {
    let taint = if a.taint == TaintStatus::Tainted || b.taint == TaintStatus::Tainted {
        TaintStatus::Tainted
    } else {
        TaintStatus::Clean
    };
    let mut sources = Vec::new();
    sources.extend(a.sources.iter().cloned());
    sources.extend(b.sources.iter().cloned());
    TaintedValue {
        value: output,
        taint,
        sources,
    }
}

pub fn sanitize<T>(val: &TaintedValue<T>, threshold_trust: f64) -> TaintedValue<&T> {
    let all_trusted = val
        .sources
        .iter()
        .all(|s| s.sensor.trust_level() >= threshold_trust);
    TaintedValue {
        value: &val.value,
        taint: if all_trusted {
            TaintStatus::Clean
        } else {
            val.taint
        },
        sources: val.sources.clone(),
    }
}

pub fn flow_through<T, U>(
    inputs: &[&TaintedValue<T>],
    _output: U,
    transform: fn(&[&TaintedValue<T>]) -> U,
) -> TaintedValue<U> {
    let any_tainted = inputs.iter().any(|v| v.taint == TaintStatus::Tainted);
    let mut sources = Vec::new();
    for v in inputs {
        sources.extend(v.sources.iter().cloned());
    }
    TaintedValue {
        value: transform(inputs),
        taint: if any_tainted {
            TaintStatus::Tainted
        } else {
            TaintStatus::Clean
        },
        sources,
    }
}
