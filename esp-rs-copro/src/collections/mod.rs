/// This module provides [`LPVec<T>`][lpvec::LPVec], which is LP-supported `Vec<T>`.
pub mod lpvec;
/// This module provides [`LPVecCopy<T>`][lpveccopy::LPVecCopy], which is a wrapper around [`LPVec<LPAdapter<T>>`][lpvec::LPVec] for `T` implementing [`Copy`].
pub mod lpveccopy;