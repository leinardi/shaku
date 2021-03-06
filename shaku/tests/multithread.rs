#![allow(clippy::blacklisted_name, clippy::mutex_atomic)]

use rand::Rng;
use shaku::{module, Component, HasComponent, Interface};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

trait Foo: Interface {
    fn get_value(&self) -> usize;
    fn set_value(&mut self, _: usize);
}

#[derive(Component)]
#[shaku(interface = Foo)]
struct FooImpl {
    #[shaku(default = FOO_DEFAULT_VALUE)]
    value: usize,
}

impl Foo for FooImpl {
    fn get_value(&self) -> usize {
        self.value
    }

    fn set_value(&mut self, val: usize) {
        self.value = val;
    }
}

module! {
    FooModule {
        components = [FooImpl],
        providers = []
    }
}

const FOO_DEFAULT_VALUE: usize = 17;
const NB_THREADS: usize = 10;
const MAX_SLEEP_TIME: u64 = 2000;

#[test]
fn simple_multithreaded_resolve_ref() {
    // Build module
    let module = FooModule::builder().build();
    let shared_module = Arc::new(Mutex::new(module));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();
    for i in 0..NB_THREADS {
        let shared_module = shared_module.clone(); // local clones to be moved into the thread

        handles.push(
            thread::Builder::new()
                .name(format!("reader #{}", &i))
                .spawn(move || {
                    // Inject some randomness in the test
                    let sleep =
                        Duration::from_millis(rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                    let handle = thread::current();
                    thread::sleep(sleep);

                    // Get a handle on the module
                    {
                        let module = shared_module.lock().unwrap();
                        let foo: &dyn Foo = module.resolve_ref();
                        assert_eq!(foo.get_value(), 17);
                        println!(
                            "In thread {:?} > resolve ok > value = {}",
                            &handle.name().unwrap(),
                            foo.get_value()
                        );
                    } // release the lock
                })
                .unwrap(),
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles
            .remove(0)
            .join()
            .unwrap_or_else(|_| panic!("Couldn't join thread {}", i));
    }
}

#[test]
fn simple_multithreaded_resolve_ref_n_mut() {
    // Build module
    let module = FooModule::builder().build();
    let shared_module = Arc::new(Mutex::new(module));
    let latest_data: Arc<Mutex<usize>> = Arc::new(Mutex::new(FOO_DEFAULT_VALUE));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();
    for i in 0..NB_THREADS {
        let (shared_module, latest_data) = (shared_module.clone(), latest_data.clone()); // local clones to be moved into the thread

        handles.push(
            thread::Builder::new()
                .name(format!("reader #{}", &i))
                .spawn(move || {
                    // Inject some randomness in the test
                    let sleep =
                        Duration::from_millis(rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                    let handle = thread::current();
                    thread::sleep(sleep);

                    // Resolve the module
                    let use_mut = rand::thread_rng().gen_range(0, 10) < 5;
                    {
                        let mut module = shared_module.lock().unwrap();

                        if use_mut {
                            let foo: &mut dyn Foo = module.resolve_mut().unwrap();
                            let new_value: usize = rand::thread_rng().gen_range(0, 256);
                            foo.set_value(new_value);
                            assert_eq!(foo.get_value(), new_value);

                            let mut data = latest_data.lock().unwrap();
                            *data = new_value;

                            println!(
                                "In thread {:?} > resolve ok > value changed to {}",
                                &handle.name().unwrap(),
                                foo.get_value()
                            );
                        } else {
                            let foo: &dyn Foo = module.resolve_ref();
                            let data = latest_data.lock().unwrap();

                            println!(
                                "In thread {:?} > resolve ok > value should be {}",
                                &handle.name().unwrap(),
                                *data
                            );
                            assert_eq!(foo.get_value(), *data);
                        }
                    } // release the lock
                })
                .unwrap(),
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles
            .remove(0)
            .join()
            .unwrap_or_else(|_| panic!("Couldn't join thread {}", i));
    }
}

#[test]
fn simple_multithreaded_resolve_n_own() {
    // Build module
    let module = FooModule::builder().build();
    let shared_module = Arc::new(Mutex::new(module));
    let latest_data: Arc<Mutex<usize>> = Arc::new(Mutex::new(FOO_DEFAULT_VALUE));

    // Launch a few threads where each will try to resolve `Foo`
    let mut handles = Vec::new();
    let owner = rand::thread_rng().gen_range(0, 10);
    println!("Owner is {}", owner);

    for i in 0..NB_THREADS {
        let (shared_module, latest_data) = (shared_module.clone(), latest_data.clone()); // local clones to be moved into the thread

        handles.push(
            thread::Builder::new()
                .name(format!("reader #{}", &i))
                .spawn(move || {
                    // Inject some randomness in the test
                    let sleep =
                        Duration::from_millis(rand::thread_rng().gen_range(0, MAX_SLEEP_TIME));
                    let handle = thread::current();
                    thread::sleep(sleep);

                    // Resolve the module
                    if i == owner {
                        let module = shared_module.lock().unwrap();
                        let foo: Arc<dyn Foo> = module.resolve();
                        let data = latest_data.lock().unwrap();
                        println!(
                            "In thread {:?} > owner > resolve ok > value should be {}",
                            &handle.name().unwrap(),
                            *data
                        );
                        assert_eq!(foo.get_value(), *data);
                    } else if i != owner {
                        let use_mut = rand::thread_rng().gen_range(0, 10) < 5;
                        {
                            let mut module = shared_module.lock().unwrap();

                            if use_mut {
                                let foo: &mut dyn Foo = module.resolve_mut().unwrap();
                                let new_value: usize = rand::thread_rng().gen_range(0, 256);
                                foo.set_value(new_value);
                                assert_eq!(foo.get_value(), new_value);

                                let mut data = latest_data.lock().unwrap();
                                *data = new_value;

                                println!(
                                    "In thread {:?} > resolve ok > value changed to {}",
                                    &handle.name().unwrap(),
                                    foo.get_value()
                                );
                            } else {
                                let foo: &dyn Foo = module.resolve_ref();
                                let data = latest_data.lock().unwrap();

                                println!(
                                    "In thread {:?} > resolve ok > value should be {}",
                                    &handle.name().unwrap(),
                                    *data
                                );
                                assert_eq!(foo.get_value(), *data);
                            }
                        } // release the lock
                    }
                })
                .unwrap(),
        );
    }

    // Wait until all the threads are done
    for i in 0..NB_THREADS {
        handles
            .remove(0)
            .join()
            .unwrap_or_else(|_| panic!("Couldn't join thread {}", i));
    }
}
