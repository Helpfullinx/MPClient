use std::time::Duration;
use avian3d::prelude::{GravityScale, LinearVelocity, Physics, PhysicsSchedule, Position};
use bevy::prelude::*;

#[test]
fn physics_test() {
    let mut app = App::new();
    
    
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin::default(),
        AssetPlugin::default(),
        MeshPlugin,
        ScenePlugin,
        PhysicsPlugins::default(),
    ));
    
    app.insert_resource(CollisionDiagnostics::default());
    app.insert_resource(SolverDiagnostics::default());
    app.insert_resource(SpatialQueryDiagnostics::default());
    
    app.add_systems(
        FixedUpdate,
        step_physics
    );
        
    app.world_mut().spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0,1.0,1.0),
        LinearVelocity::default(),
        LockedAxes::new().lock_rotation_x().lock_rotation_y().lock_rotation_z(),
        Position::default(),
        Rotation::default(),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.0)),
        GravityScale::default()
    ));
    
    let mut transform = app.world_mut().query::<(&Position, &LinearVelocity, &GravityScale)>();
    for t in transform.iter(app.world()){
        println!("{:?}", t);
    }
    for _ in 0..10 {
        sleep(Duration::from_secs_f64(1.0 / 60.0));
        let time = app.world_mut().resource_mut::<Time<Physics>>();
        println!("{:?}", time.delta());
        app.update();
    }

    for t in transform.iter(app.world()){
        println!("{:?}", t);
    }
}

fn step_physics(world: &mut World) {
    for _ in 0..10 {
        println!("Physics stepped");

        let mut transform = world.query::<(&Position, &LinearVelocity, &GravityScale)>();
        for t in transform.iter(world){
            println!("{:?}", t);
        }

        let mut time = world.resource_mut::<Time<Physics>>();
        // println!("{:?}", time);
        time.advance_by(Duration::from_secs_f64(1.0/64.0));
        world.run_schedule(PhysicsSchedule);

        let mut transform = world.query::<(&Position, &LinearVelocity, &GravityScale)>();
        for t in transform.iter(world){
            println!("{:?}", t);
        }


    }
}