use std::{time};
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::{random_bool, random_range, rng, Rng};

#[derive(Debug, Copy, Clone)]
pub struct Particle {
    x: f32,
    y: f32,
    id: usize
}
impl Particle {
    pub fn new(x_param:f32, y_param:f32, id_param: usize) -> Particle {
        Particle {
            x: x_param,
            y: y_param,
            id:id_param
        }
    }
    pub fn collide(&self, other: &Particle) -> bool {
        let x = other.x - self.x;
        let y = other.y - self.y;
        return x * x + y * y <= 0.25 * 0.25;
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    collision_counter: Arc<AtomicUsize>
}
impl ParticleSystem {
    pub fn new()-> ParticleSystem {
        ParticleSystem {
            particles: Vec::new(),
            collision_counter: Arc::new(AtomicUsize::new(0))
        }
    }
    pub fn move_particles_loop(&mut self) {
        // Loop and measure time. Without print statements, roughly 6,000,000 loops equates to 10 seconds (Ryzen 5 7600x).
        // With print statements, the loop count drastically decreases to around 2000.
        let num_iterations = 20000;
        // let num_iterations = 6000000;
        let thread_count = 12;
        let particles_per_thread = self.particles.len() / thread_count;
        println!("Moving {} particles {} times across {} threads...", self.particles.len(), num_iterations, thread_count);
        let start_time = time::Instant::now();

        // Initialise threads
        let mut pool = scoped_threadpool::Pool::new(thread_count as u32);
        pool.scoped(|scope| {
            let mut i = 0usize;
            for chunk in self.particles.chunks_mut(particles_per_thread) {
                scope.execute(move || thread_main(chunk, num_iterations, i));
                i += 1;
            }
        });

        let duration = time::Instant::now().duration_since(start_time);
        println!("Took {} ms to move {} particles {} times", duration.as_millis(), self.particles.len(), num_iterations);
    }
    pub fn collide_particles(&mut self) {
        let list_len = self.particles.len();
        let thread_count = 12;
        let particles_per_thread = list_len / thread_count;
        let mut collision_pool = scoped_threadpool::Pool::new(thread_count as u32);

        println!("Checking collisions...");
        let start_time = time::Instant::now();

        collision_pool.scoped(|scope| {
            let mut thread_id = 0usize;

            for i in 0..thread_count {
                let clone = self.particles.clone();
                let counter_clone = Arc::clone(&self.collision_counter);
                scope.execute(move || thread_collide(&clone, &counter_clone, particles_per_thread, thread_id));
                thread_id += 1;
            }
        });

        let duration = time::Instant::now().duration_since(start_time);
        println!("Took {} ms to check collisions. Detected {} collisions", duration.as_millis(), self.collision_counter.load(Ordering::Relaxed));
    }
    pub fn move_and_collide_particles(&mut self) {
        let num_iterations = 125000;
        let num_threads_total = 12;
        let num_threads_movement = 2;
        let num_threads_collision = num_threads_total - num_threads_movement;
        let num_particles_total = self.particles.len();
        let num_particles_movement = num_particles_total / num_threads_movement;
        let num_particles_collision = num_particles_total / num_threads_collision;

        let start_time = time::Instant::now();

        // Set up thread pools
        let mut pool_movement = scoped_threadpool::Pool::new(num_threads_movement as u32);
        let mut pool_collision = scoped_threadpool::Pool::new(num_threads_collision as u32);

        // Iteratively run threads
        for i in 0..num_iterations {
            // Run movement threads
            // println!("Moving {} particles across {} threads...", self.particles.len(), num_threads_movement);
            pool_movement.scoped(|scope| {
                let mut thread_id = 0usize;
                for chunk in self.particles.chunks_mut(num_particles_movement) {
                    scope.execute(move || thread_main(chunk, 1, thread_id));
                    thread_id += 1;
                }
            });

            // Run collision threads
            // println!("Checking collisions across {} threads...", num_threads_collision);
            pool_collision.scoped(|scope| {
                let mut thread_id = 0usize;
                for i in 0..num_threads_collision {
                    let list_clone = self.particles.clone();
                    let collision_counter_clone = Arc::clone(&self.collision_counter);
                    scope.execute(move || thread_collide(&list_clone, &collision_counter_clone, num_particles_collision, thread_id));
                    thread_id += 1;
                }
            });
        }

        let duration = time::Instant::now().duration_since(start_time);
        println!("Took {} ms to move {} particles & check collisions over {} iterations.", duration.as_millis(), num_particles_total, num_iterations);
        println!("Detected {} collisions in total.", self.collision_counter.load(Ordering::Relaxed));
    }
}

const PARTICLE_COUNT:usize = 100;
const PARTICLE_BOUNDS:(i32, i32) = (10, 10);
const PARTICLE_BOUNDS_HALF:(f32, f32) = (PARTICLE_BOUNDS.0 as f32 * 0.5, PARTICLE_BOUNDS.1 as f32 * 0.5);

fn main() {
    // Create particle system object
    let mut particle_system = ParticleSystem::new();

    // Create particles & add to system
    println!("Creating {} particles...", PARTICLE_COUNT);
    for i in 0..PARTICLE_COUNT {
        // Generate random positions within bounds
        let x = random_range(-PARTICLE_BOUNDS_HALF.0..PARTICLE_BOUNDS_HALF.0);
        let y = random_range(-PARTICLE_BOUNDS_HALF.1..PARTICLE_BOUNDS_HALF.1);

        // Create instance with generated position
        let particle = Particle::new(x, y, i);

        // Announce position
        // println!("Created particle {} with position ({}, {})", i, particle.x, particle.y);

        // Add instance to system
        particle_system.particles.push(particle);
    }

    // Run loop
    // particle_system.move_particles_loop();
    // particle_system.collide_particles();

    particle_system.move_and_collide_particles();
}
fn thread_main(chunk: &mut [Particle], iteration_count: i32, thread_index: usize) {
    let chunk_size = chunk.len();
    let mut rng = rng();
    for j in 0..iteration_count {
        // println!("Thread {} moving particles...", thread_index);
        for i in 0..chunk_size as usize {
            // Generate vector to add and decide whether or not it should be negative (50% chance)
            let mut xy = (rng.random::<f32>(), rng.random::<f32>());
            let negative = (random_bool(0.5), random_bool(0.5));
            if (negative.0) {
                xy.0 = -xy.0;
            }
            if (negative.1) {
                xy.1 = -xy.1;
            }

            // Apply vector to particle
            chunk[i].x += xy.0;
            chunk[i].y += xy.1;

            // Restrict particle to within declared boundaries
            if (chunk[i].x < -PARTICLE_BOUNDS_HALF.0){
                chunk[i].x = -PARTICLE_BOUNDS_HALF.0;
            }
            else if (chunk[i].x > PARTICLE_BOUNDS_HALF.0) {
                chunk[i].x = PARTICLE_BOUNDS_HALF.0;
            }

            if (chunk[i].y < -PARTICLE_BOUNDS_HALF.1){
                chunk[i].y = -PARTICLE_BOUNDS_HALF.1;
            }
            else if (chunk[i].y > PARTICLE_BOUNDS_HALF.1) {
                chunk[i].y = PARTICLE_BOUNDS_HALF.1;
            }

            // println!("Particle {} moved. New position: ({}, {})", i + (chunk_size * thread_index), chunk[i].x, chunk[i].y);
        }
    }
}
fn thread_collide(list: &Vec<Particle>, collision_count: &AtomicUsize, particles_per_thread: usize, thread_id: usize) {
    let mut local_collision_count = 0;
    let start_time = time::Instant::now();
    let list_size = list.len();

    let mut clone = list.clone();
    let chunk = clone.chunks_mut(particles_per_thread).nth(thread_id).unwrap();

    for i in 0..particles_per_thread - 1 {
        let i_id = chunk[i].id;
            for j in i_id + 1..list_size {
                // Skip if comparing to self
                if (i_id == j) {
                    continue;
                }

                if (chunk[i].collide(&list[j])) {
                    local_collision_count += 1;
                    collision_count.fetch_add(1, Ordering::Relaxed);
                    // println!("Collision found between particles {} ({}, {}) and {} ({}, {})", i_id, chunk[i].x, chunk[i].y, j, list[j].x, list[j].y);
                }
        }
    }

    let duration = time::Instant::now().duration_since(start_time);
    // println!("Thread {} spent {} ms on collision checking, and detected {} total collisions", thread_id, duration.as_millis(), local_collision_count);
}