use std::{
	ops::{
		Deref,
		DerefMut
	},
	borrow::{
		Borrow,
		BorrowMut
	},
	convert::{
		AsRef,
		AsMut
	},
	sync::{
		Arc,
		Weak
	},
	hash::{
		Hash,
		Hasher
	},
	marker::PhantomData
};
use slab::Slab;
use crossbeam_queue::SegQueue;
use crate::Event;

struct State {
	grabs: SegQueue<usize>,
	releases: SegQueue<usize>
}

pub struct Scene<T, E> {
	/// Scene objects.
	slab: Slab<Entry<T>>,

	/// Grabs and releases.
	state: Arc<State>,

	/// Event emitted during the last cycle.
	events: Vec<E>
}

impl<T, E> Scene<T, E> {
	pub fn new() -> Scene<T, E> {
		Scene {
			slab: Slab::new(),
			state: Arc::new(State {
				grabs: SegQueue::new(),
				releases: SegQueue::new()
			}),
			events: Vec::new()
		}
	}

	#[inline]
	pub fn events(&self) -> &[E] {
		&self.events
	}

	#[inline]
	pub fn clear_events(&mut self) {
		self.events.clear()
	}

	pub fn id(&self, index: usize) -> Option<Id<T>> {
		self.slab.get(index).map(|_| {
			self.state.grabs.push(index);
			Id(self.state.clone(), index, PhantomData)
		})
	}

	pub fn get<'a>(&'a self, id: &'a Id<T>) -> Ref<'a, T> {
		assert!(Arc::ptr_eq(&id.0, &self.state));
		let entry = self.slab.get(id.1).unwrap();
		Ref {
			entry, id
		}
	}

	pub fn get_mut<'a>(&'a mut self, id: &'a Id<T>) -> Mut<'a, T> {
		assert!(Arc::ptr_eq(&id.0, &self.state));
		let entry = self.slab.get_mut(id.1).unwrap();
		Mut {
			entry, id
		}
	}
}

impl<T, E> Scene<T, E> where Event: Into<E> {
	/// Remove unused entities.
	/// 
	/// Use must call this function from time to time
	/// to limit memory usage.
	/// 
	/// This may emit new `Drop` events,
	/// so be sure not to call `clear_events` before
	/// having handled those events.
	pub fn garbage_collect(&mut self) {
		while let Some(id) = self.state.grabs.pop() {
			self.slab.get_mut(id).unwrap().grab()
		}

		while let Some(id) = self.state.releases.pop() {
			if self.slab.get_mut(id).unwrap().release() {
				self.slab.remove(id);
				self.events.push(Event::Drop(id).into());
			}
		}
	}

	pub fn insert(&mut self, t: T) -> Id<T> {
		let id = self.slab.insert(Entry {
			data: t,
			refs: 1
		});
		self.events.push(Event::New(id).into());
		Id(self.state.clone(), id, PhantomData)
	}
}

struct Entry<T> {
	data: T,
	refs: usize
}

impl<T> Entry<T> {
	#[inline]
	fn grab(&mut self) {
		self.refs += 1
	}

	#[inline]
	fn release(&mut self) -> bool {
		self.refs -= 1;
		self.refs == 0
	}
}

impl<T> Drop for Entry<T> {
	#[inline]
	fn drop(&mut self) {
		assert!(self.refs == 0)
	}
}

#[derive(Clone)]
pub struct Ref<'a, T> {
	entry: &'a Entry<T>,
	id: &'a Id<T>
}

impl<'a, T> Ref<'a, T> {
	#[inline]
	pub fn id(&self) -> &'a Id<T> {
		self.id
	}
}

impl<'a, T> Deref for Ref<'a, T> {
	type Target = T;

	#[inline]
	fn deref(&self) -> &T {
		&self.entry.data
	}
}

impl<'a, T> Borrow<T> for Ref<'a, T> {
	#[inline]
	fn borrow(&self) -> &T {
		&self.entry.data
	}
}

impl<'a, T> AsRef<T> for Ref<'a, T> {
	#[inline]
	fn as_ref(&self) -> &T {
		&self.entry.data
	}
}

pub struct Mut<'a, T> {
	entry: &'a mut Entry<T>,
	id: &'a Id<T>
}

impl<'a, T> Mut<'a, T> {
	#[inline]
	pub fn id(&self) -> &'a Id<T> {
		self.id
	}
}

impl<'a, T> Deref for Mut<'a, T> {
	type Target = T;

	#[inline]
	fn deref(&self) -> &T {
		&self.entry.data
	}
}

impl<'a, T> DerefMut for Mut<'a, T> {
	#[inline]
	fn deref_mut(&mut self) -> &mut T {
		&mut self.entry.data
	}
}

impl<'a, T> Borrow<T> for Mut<'a, T> {
	#[inline]
	fn borrow(&self) -> &T {
		&self.entry.data
	}
}

impl<'a, T> BorrowMut<T> for Mut<'a, T> {
	#[inline]
	fn borrow_mut(&mut self) -> &mut T {
		&mut self.entry.data
	}
}

impl<'a, T> AsRef<T> for Mut<'a, T> {
	#[inline]
	fn as_ref(&self) -> &T {
		&self.entry.data
	}
}

impl<'a, T> AsMut<T> for Mut<'a, T> {
	#[inline]
	fn as_mut(&mut self) -> &mut T {
		&mut self.entry.data
	}
}

pub struct Id<T>(Arc<State>, usize, PhantomData<T>);

impl<T> Id<T> {
	#[inline]
	pub fn index(&self) -> usize {
		self.1
	}

	#[inline]
	pub fn downgrade(&self) -> WeakId<T> {
		WeakId(Arc::downgrade(&self.0), self.1, PhantomData)
	}
}

impl<T> Clone for Id<T> {
	#[inline]
	fn clone(&self) -> Id<T> {
		self.0.grabs.push(self.1);
		Id(self.0.clone(), self.1, PhantomData)
	}
}

impl<T> Drop for Id<T> {
	#[inline]
	fn drop(&mut self) {
		self.0.releases.push(self.1)
	}
}

impl<T> PartialEq for Id<T> {
	#[inline]
	fn eq(&self, other: &Id<T>) -> bool {
		Arc::ptr_eq(&self.0, &other.0) && self.1 == other.1
	}
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
	#[inline]
	fn hash<H: Hasher>(&self, h: &mut H) {
		self.1.hash(h)
	}
}

pub struct WeakId<T>(Weak<State>, usize, PhantomData<T>);

impl<T> WeakId<T> {
	#[inline]
	pub fn index(&self) -> usize {
		self.1
	}

	#[inline]
	pub fn upgrade<E>(&self, lib: &Scene<T, E>) -> Option<Id<T>> {
		let arc = self.0.upgrade().unwrap();
		lib.slab.get(self.1).map(|_| Id(arc, self.1, PhantomData))
	}
}

impl<T> Clone for WeakId<T> {
	#[inline]
	fn clone(&self) -> WeakId<T> {
		WeakId(self.0.clone(), self.1, PhantomData)
	}
}

impl<T> PartialEq for WeakId<T> {
	#[inline]
	fn eq(&self, other: &WeakId<T>) -> bool {
		Weak::ptr_eq(&self.0, &other.0) && self.1 == other.1
	}
}

impl<T> Eq for WeakId<T> {}

impl<T> Hash for WeakId<T> {
	#[inline]
	fn hash<H: Hasher>(&self, h: &mut H) {
		self.1.hash(h)
	}
}