use crate::pipeline::{Capability, Node, NodeRef};

use super::Error;

pub fn validate_inputs<I, V>(inputs: I, validate: V) -> Result<V::Validated, Error>
where
    I: IntoIterator<Item = NodeRef>,
    V: Validate,
{
    validate.validate(inputs)
}

pub trait Validator {
    fn check(&self, input: &NodeRef) -> bool;
}

impl Validator for Capability {
    fn check(&self, input: &NodeRef) -> bool {
        input.has_capability(*self)
    }
}

impl Validator for [Capability; 2] {
    fn check(&self, input: &NodeRef) -> bool {
        check_slice(&self[..], input)
    }
}

fn check_slice(caps: &[Capability], input: &NodeRef) -> bool {
    caps.iter().copied().any(|cap| input.has_capability(cap))
}

pub trait Validate: private::Sealed {
    type Validated;

    fn validate<I: IntoIterator<Item = NodeRef>>(
        &self,
        inputs: I,
    ) -> Result<Self::Validated, Error>;
}

impl Validate for () {
    type Validated = ();

    fn validate<I: IntoIterator<Item = NodeRef>>(
        &self,
        inputs: I,
    ) -> Result<Self::Validated, Error> {
        let mut inputs = inputs.into_iter();

        if inputs.next().is_some() {
            return Err(Error::InvalidInputs);
        }

        Ok(())
    }
}

impl<T> Validate for T
where
    T: Validator,
{
    type Validated = NodeRef;

    fn validate<I: IntoIterator<Item = NodeRef>>(
        &self,
        inputs: I,
    ) -> Result<Self::Validated, Error> {
        let mut inputs = inputs.into_iter();
        let input = inputs.next().ok_or(Error::InvalidInputs)?;

        if inputs.next().is_some() {
            return Err(Error::InvalidInputs);
        }

        self.check(&input)
            .then_some(input)
            .ok_or(Error::InvalidInputs)
    }
}

impl<T1, T2> Validate for (T1, T2)
where
    T1: Validator,
    T2: Validator,
{
    type Validated = (NodeRef, NodeRef);

    fn validate<I: IntoIterator<Item = NodeRef>>(
        &self,
        inputs: I,
    ) -> Result<Self::Validated, Error> {
        let mut inputs = inputs.into_iter();
        let mut validators = [&self.0 as &dyn Validator, &self.1 as &dyn Validator].into_iter();

        let mut validated = Vec::new();

        for (input, validator) in (&mut inputs).zip(&mut validators) {
            if !validator.check(&input) {
                return Err(Error::InvalidInputs);
            }

            validated.push(input);
        }

        if inputs.next().is_some() || validators.next().is_some() {
            // Invalid number of inputs (either less or more).
            return Err(Error::InvalidInputs);
        }

        Ok((validated[0].clone(), validated[1].clone()))
    }
}

mod private {
    use super::Validator;

    pub trait Sealed {}

    impl Sealed for () {}
    impl<T> Sealed for T where T: Validator {}
    impl<T1, T2> Sealed for (T1, T2)
    where
        T1: Validator,
        T2: Validator,
    {
    }
}
