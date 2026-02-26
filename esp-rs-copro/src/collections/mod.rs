/// Growable collection types designed for LP memory. The primary collection is [`LPVec<T>`][lpvec::LPVec] for movable objects.
pub mod lpvec;
/// This module provides [`LPVecCopy<T>`][lpveccopy::LPVecCopy], which is a wrapper around [`LPVec<LPAdapter<T>>`][lpvec::LPVec] for `T` implementing [`Copy`].
pub mod lpveccopy;