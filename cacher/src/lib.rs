use std::{cmp, collections::HashMap, hash::Hash};

pub struct Cacher<T, U, V>
where
    T: Fn(U) -> V,
    U: Clone + cmp::Eq + Hash,
    V: Clone,
{
    pub calculation: T,
    pub values: HashMap<U, V>,
}

impl<T, U, V> Cacher<T, U, V>
where
    T: Fn(U) -> V,
    U: Clone + cmp::Eq + Hash,
    V: Clone,
{
    pub fn new(calculation: T) -> Cacher<T, U, V> {
        Cacher {
            calculation,
            values: HashMap::new(),
        }
    }

    pub fn value(&mut self, arg: U) -> &V {
        self.values
            .entry(arg.clone())
            .or_insert((self.calculation)(arg.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_with_different_values() {
        let mut c = Cacher::new(|a| a);

        let _v1 = c.value(1);
        let v2 = c.value(2);

        assert_eq!(*v2, 2);
    }
}
