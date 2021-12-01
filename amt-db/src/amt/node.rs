use crate::crypto::export::{AffineCurve, FromBytes, ProjectiveCurve, Read, ToBytes, Write};
use crate::crypto::TypeUInt;
use crate::serde::{MyFromBytes, MyToBytes, SerdeType};
use std::io::Result as IoResult;
use std::marker::PhantomData;

#[derive(Clone, Copy, Default)]
pub struct AMTNode<G: ProjectiveCurve> {
    pub commitment: G,
    pub proof: G,
}

impl<G: ProjectiveCurve> MyFromBytes for AMTNode<G> {
    fn read<R: Read>(mut reader: R, ty: SerdeType) -> IoResult<Self> {
        if ty.consistent {
            let g1_aff: <G as ProjectiveCurve>::Affine = FromBytes::read(&mut reader)?;
            let g2_aff: <G as ProjectiveCurve>::Affine = FromBytes::read(&mut reader)?;
            Ok(Self {
                commitment: g1_aff.into_projective(),
                proof: g2_aff.into_projective(),
            })
        } else {
            Ok(Self {
                commitment: FromBytes::read(&mut reader)?,
                proof: FromBytes::read(&mut reader)?,
            })
        }
    }
}

impl<G: ProjectiveCurve> MyToBytes for AMTNode<G> {
    fn write<W: Write>(&self, mut writer: W, ty: SerdeType) -> IoResult<()> {
        if ty.consistent {
            ToBytes::write(&self.commitment.into_affine(), &mut writer)?;
            ToBytes::write(&self.proof.into_affine(), &mut writer)?;
        } else {
            ToBytes::write(&self.commitment, &mut writer)?;
            ToBytes::write(&self.proof, &mut writer)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NodeIndex<N: TypeUInt> {
    depth: usize,
    index: usize,
    _phantom: PhantomData<N>,
}

impl<N: TypeUInt> NodeIndex<N> {
    #[inline]
    pub(crate) fn new(depth: usize, index: usize) -> Self {
        assert!(index < (1 << depth));
        assert!(depth <= N::USIZE);
        Self {
            depth,
            index,
            _phantom: PhantomData,
        }
    }

    pub fn root() -> Self {
        NodeIndex::new(0, 0)
    }

    #[inline]
    pub fn to_sibling(&self) -> Self {
        NodeIndex::new(self.depth, self.index ^ 1)
    }

    #[inline]
    pub fn to_ancestor(&self, height: usize) -> Self {
        assert!(height <= self.depth);
        NodeIndex::new(self.depth - height, self.index >> height)
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn total_depth(&self) -> usize {
        N::USIZE
    }
}
