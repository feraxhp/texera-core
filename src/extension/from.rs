use super::Extension;


impl<T> From<T> for Extension
where 
    T: super::Facet
{
    fn from(value: T) -> Self {
        Extension::facet::<T>(value)
    }
}

impl<const N: usize> From<[Self; N]> for Extension {
    fn from(value: [Self; N]) -> Self { 
        Self::Group(value.to_vec()) 
    }
}

impl From<Vec<Self>> for Extension {
    fn from(value: Vec<Self>) -> Self {
        Self::Group(value)
    }
}
