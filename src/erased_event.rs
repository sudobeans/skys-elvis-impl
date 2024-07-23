
impl ErasedEvent {
    fn new<N: Node>(receiver: Arc<Mutex<N>>, event: CallbackEvent<N>) -> ErasedEvent {
        let f = move || {
            let evs = (event.event)(receiver.lock().unwrap().borrow_mut());
            let mut erased_evs = Vec::with_capacity(evs.capacity());
            for ev in evs {
                let erased = ErasedEvent::new(receiver.clone(), ev);
                erased_evs.push(erased);
            }
            erased_evs
        };
        ErasedEvent {
            time: event.time,
            event: Box::new(f),
        }
    }
}