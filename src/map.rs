use std::marker::PhantomData;
use crate::{
	Id,
	WeakId
};

pub trait Key<K>: Copy {
	fn index(&self) -> usize;
}

impl<'a, K> Key<K> for &'a Id<K> {
	#[inline]
	fn index(&self) -> usize {
		Id::index(self)
	}
}

impl<'a, K> Key<K> for &'a WeakId<K> {
	#[inline]
	fn index(&self) -> usize {
		WeakId::index(self)
	}
}

impl<'a, K> Key<K> for usize {
	#[inline]
	fn index(&self) -> usize {
		*self
	}
}

pub struct Map<K, T> {
	data: Vec<Option<T>>,
	k: PhantomData<K>
}

impl<K, T> Map<K, T> {
	pub fn new() -> Map<K, T> {
		Map {
			data: Vec::new(),
			k: PhantomData
		}
	}

	pub fn get<I>(&self, id: I) -> Option<&T> where I: Key<K> {
		self.data.get(id.index()).map(|o| o.as_ref()).flatten()
	}

	pub fn get_mut<I>(&mut self, id: I) -> Option<&mut T> where I: Key<K> {
		self.data.get_mut(id.index()).map(|o| o.as_mut()).flatten()
	}

	pub fn set<I>(&mut self, id: I, t: T) -> Option<T> where I: Key<K> {
		if self.data.len() <= id.index() {
			self.data.resize_with(id.index()+1, || None);
		}
		
		let mut result = Some(t);
		std::mem::swap(&mut self.data[id.index()], &mut result);
		result
	}
}