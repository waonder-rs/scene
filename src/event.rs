pub enum Event {
	/// A new object has been inserted in the scene.
	New(usize),

	/// An object has been dropped.
	/// 
	/// This event will always be emitted before any `New` event with the same id
	/// that would replace the recently dropped object.
	Drop(usize)
}