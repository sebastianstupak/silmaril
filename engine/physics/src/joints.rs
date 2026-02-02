//! Physics joints and constraints
//!
//! This module provides a high-level API for creating and managing physics joints.
//! Joints constrain the relative motion between two rigid bodies.
//!
//! # Joint Types
//!
//! - **Fixed**: Bodies maintain their relative position and orientation
//! - **Revolute**: Bodies can rotate around a single axis (hinge)
//! - **Prismatic**: Bodies can slide along a single axis (slider)
//! - **Spherical**: Bodies can rotate freely (ball-and-socket)
//!
//! # Example
//!
//! ```rust
//! use engine_physics::{PhysicsWorld, Joint, JointBuilder};
//! use engine_math::Vec3;
//!
//! // Create a revolute joint (hinge)
//! let joint = JointBuilder::revolute()
//!     .axis(Vec3::Y)
//!     .anchor1(Vec3::new(0.0, 1.0, 0.0))
//!     .anchor2(Vec3::new(0.0, -1.0, 0.0))
//!     .limits(-1.57, 1.57)  // ±90 degrees
//!     .build();
//! ```

use engine_math::Vec3;
use rapier3d::na::Unit;
use rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

/// Joint handle - uniquely identifies a joint in the physics world
pub type JointHandle = ImpulseJointHandle;

/// Joint type and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Joint {
    /// Fixed joint - bodies maintain relative position and orientation
    Fixed(FixedJointConfig),

    /// Revolute joint - bodies rotate around a shared axis (hinge)
    Revolute(RevoluteJointConfig),

    /// Prismatic joint - bodies slide along a shared axis (slider)
    Prismatic(PrismaticJointConfig),

    /// Spherical joint - bodies rotate freely around a point (ball-and-socket)
    Spherical(SphericalJointConfig),
}

/// Configuration for a fixed joint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedJointConfig {
    /// Anchor point on body 1 (local space)
    pub anchor1: Vec3,

    /// Anchor point on body 2 (local space)
    pub anchor2: Vec3,
}

/// Configuration for a revolute joint (hinge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevoluteJointConfig {
    /// Anchor point on body 1 (local space)
    pub anchor1: Vec3,

    /// Anchor point on body 2 (local space)
    pub anchor2: Vec3,

    /// Rotation axis (local space)
    pub axis: Vec3,

    /// Angle limits (min, max) in radians
    pub limits: Option<(f32, f32)>,

    /// Motor configuration
    pub motor: Option<JointMotor>,
}

/// Configuration for a prismatic joint (slider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismaticJointConfig {
    /// Anchor point on body 1 (local space)
    pub anchor1: Vec3,

    /// Anchor point on body 2 (local space)
    pub anchor2: Vec3,

    /// Sliding axis (local space)
    pub axis: Vec3,

    /// Distance limits (min, max) in meters
    pub limits: Option<(f32, f32)>,

    /// Motor configuration
    pub motor: Option<JointMotor>,
}

/// Configuration for a spherical joint (ball-and-socket)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphericalJointConfig {
    /// Anchor point on body 1 (local space)
    pub anchor1: Vec3,

    /// Anchor point on body 2 (local space)
    pub anchor2: Vec3,
}

/// Joint motor configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointMotor {
    /// Target velocity (rad/s for revolute, m/s for prismatic)
    pub target_velocity: f32,

    /// Maximum force the motor can apply (N or N·m)
    pub max_force: f32,

    /// Stiffness coefficient (0.0 = soft, higher = stiffer)
    pub stiffness: f32,

    /// Damping coefficient
    pub damping: f32,
}

impl Default for JointMotor {
    fn default() -> Self {
        Self { target_velocity: 0.0, max_force: f32::INFINITY, stiffness: 0.0, damping: 1.0 }
    }
}

impl JointMotor {
    /// Create a new motor with target velocity
    pub fn new(target_velocity: f32) -> Self {
        Self { target_velocity, ..Default::default() }
    }

    /// Set maximum force
    pub fn with_max_force(mut self, max_force: f32) -> Self {
        self.max_force = max_force;
        self
    }

    /// Set stiffness
    pub fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness;
        self
    }

    /// Set damping
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }
}

/// Builder for creating joints with a fluent API
pub struct JointBuilder {
    joint_type: JointType,
    anchor1: Vec3,
    anchor2: Vec3,
    axis: Vec3,
    limits: Option<(f32, f32)>,
    motor: Option<JointMotor>,
}

enum JointType {
    Fixed,
    Revolute,
    Prismatic,
    Spherical,
}

impl JointBuilder {
    /// Create a fixed joint builder
    pub fn fixed() -> Self {
        Self {
            joint_type: JointType::Fixed,
            anchor1: Vec3::ZERO,
            anchor2: Vec3::ZERO,
            axis: Vec3::Y,
            limits: None,
            motor: None,
        }
    }

    /// Create a revolute joint builder (hinge)
    pub fn revolute() -> Self {
        Self {
            joint_type: JointType::Revolute,
            anchor1: Vec3::ZERO,
            anchor2: Vec3::ZERO,
            axis: Vec3::Y,
            limits: None,
            motor: None,
        }
    }

    /// Create a prismatic joint builder (slider)
    pub fn prismatic() -> Self {
        Self {
            joint_type: JointType::Prismatic,
            anchor1: Vec3::ZERO,
            anchor2: Vec3::ZERO,
            axis: Vec3::Y,
            limits: None,
            motor: None,
        }
    }

    /// Create a spherical joint builder (ball-and-socket)
    pub fn spherical() -> Self {
        Self {
            joint_type: JointType::Spherical,
            anchor1: Vec3::ZERO,
            anchor2: Vec3::ZERO,
            axis: Vec3::Y,
            limits: None,
            motor: None,
        }
    }

    /// Set anchor point on body 1 (local space)
    pub fn anchor1(mut self, anchor: Vec3) -> Self {
        self.anchor1 = anchor;
        self
    }

    /// Set anchor point on body 2 (local space)
    pub fn anchor2(mut self, anchor: Vec3) -> Self {
        self.anchor2 = anchor;
        self
    }

    /// Set axis (for revolute and prismatic joints)
    pub fn axis(mut self, axis: Vec3) -> Self {
        self.axis = axis;
        self
    }

    /// Set limits (angle limits for revolute, distance limits for prismatic)
    pub fn limits(mut self, min: f32, max: f32) -> Self {
        self.limits = Some((min, max));
        self
    }

    /// Set motor configuration
    pub fn motor(mut self, motor: JointMotor) -> Self {
        self.motor = Some(motor);
        self
    }

    /// Build the joint configuration
    pub fn build(self) -> Joint {
        match self.joint_type {
            JointType::Fixed => {
                Joint::Fixed(FixedJointConfig { anchor1: self.anchor1, anchor2: self.anchor2 })
            }
            JointType::Revolute => Joint::Revolute(RevoluteJointConfig {
                anchor1: self.anchor1,
                anchor2: self.anchor2,
                axis: self.axis,
                limits: self.limits,
                motor: self.motor,
            }),
            JointType::Prismatic => Joint::Prismatic(PrismaticJointConfig {
                anchor1: self.anchor1,
                anchor2: self.anchor2,
                axis: self.axis,
                limits: self.limits,
                motor: self.motor,
            }),
            JointType::Spherical => Joint::Spherical(SphericalJointConfig {
                anchor1: self.anchor1,
                anchor2: self.anchor2,
            }),
        }
    }
}

impl Joint {
    /// Convert engine joint to Rapier impulse joint
    pub fn to_rapier(&self) -> GenericJoint {
        match self {
            Joint::Fixed(config) => {
                let mut joint = GenericJoint::new(JointAxesMask::LOCKED_FIXED_AXES);
                joint.set_local_anchor1(point![
                    config.anchor1.x,
                    config.anchor1.y,
                    config.anchor1.z
                ]);
                joint.set_local_anchor2(point![
                    config.anchor2.x,
                    config.anchor2.y,
                    config.anchor2.z
                ]);
                joint
            }
            Joint::Revolute(config) => {
                let mut joint = RevoluteJointBuilder::new(Unit::new_normalize(vector![
                    config.axis.x,
                    config.axis.y,
                    config.axis.z
                ]))
                .local_anchor1(point![config.anchor1.x, config.anchor1.y, config.anchor1.z])
                .local_anchor2(point![
                    config.anchor2.x,
                    config.anchor2.y,
                    config.anchor2.z
                ]);

                if let Some((min, max)) = config.limits {
                    joint = joint.limits([min, max]);
                }

                if let Some(motor) = &config.motor {
                    joint = joint
                        .motor_velocity(motor.target_velocity, motor.max_force)
                        .motor_model(MotorModel::ForceBased);
                }

                joint.build().into()
            }
            Joint::Prismatic(config) => {
                let mut joint = PrismaticJointBuilder::new(Unit::new_normalize(vector![
                    config.axis.x,
                    config.axis.y,
                    config.axis.z
                ]))
                .local_anchor1(point![config.anchor1.x, config.anchor1.y, config.anchor1.z])
                .local_anchor2(point![
                    config.anchor2.x,
                    config.anchor2.y,
                    config.anchor2.z
                ]);

                if let Some((min, max)) = config.limits {
                    joint = joint.limits([min, max]);
                }

                if let Some(motor) = &config.motor {
                    joint = joint
                        .motor_velocity(motor.target_velocity, motor.max_force)
                        .motor_model(MotorModel::ForceBased);
                }

                joint.build().into()
            }
            Joint::Spherical(config) => SphericalJointBuilder::new()
                .local_anchor1(point![config.anchor1.x, config.anchor1.y, config.anchor1.z])
                .local_anchor2(point![config.anchor2.x, config.anchor2.y, config.anchor2.z])
                .build()
                .into(),
        }
    }

    /// Alias for to_rapier() - converts engine joint to Rapier impulse joint
    pub fn to_rapier_joint(&self) -> GenericJoint {
        self.to_rapier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_joint_builder_fixed() {
        let joint = JointBuilder::fixed()
            .anchor1(Vec3::new(1.0, 0.0, 0.0))
            .anchor2(Vec3::new(-1.0, 0.0, 0.0))
            .build();

        match joint {
            Joint::Fixed(config) => {
                assert_eq!(config.anchor1, Vec3::new(1.0, 0.0, 0.0));
                assert_eq!(config.anchor2, Vec3::new(-1.0, 0.0, 0.0));
            }
            _ => panic!("Expected Fixed joint"),
        }
    }

    #[test]
    fn test_joint_builder_revolute() {
        let joint = JointBuilder::revolute()
            .anchor1(Vec3::new(0.0, 1.0, 0.0))
            .anchor2(Vec3::new(0.0, -1.0, 0.0))
            .axis(Vec3::Z)
            .limits(-PI / 2.0, PI / 2.0)
            .build();

        match joint {
            Joint::Revolute(config) => {
                assert_eq!(config.anchor1, Vec3::new(0.0, 1.0, 0.0));
                assert_eq!(config.anchor2, Vec3::new(0.0, -1.0, 0.0));
                assert_eq!(config.axis, Vec3::Z);
                assert_eq!(config.limits, Some((-PI / 2.0, PI / 2.0)));
            }
            _ => panic!("Expected Revolute joint"),
        }
    }

    #[test]
    fn test_joint_builder_prismatic() {
        let joint = JointBuilder::prismatic()
            .anchor1(Vec3::ZERO)
            .anchor2(Vec3::ZERO)
            .axis(Vec3::X)
            .limits(-5.0, 5.0)
            .build();

        match joint {
            Joint::Prismatic(config) => {
                assert_eq!(config.anchor1, Vec3::ZERO);
                assert_eq!(config.anchor2, Vec3::ZERO);
                assert_eq!(config.axis, Vec3::X);
                assert_eq!(config.limits, Some((-5.0, 5.0)));
            }
            _ => panic!("Expected Prismatic joint"),
        }
    }

    #[test]
    fn test_joint_builder_spherical() {
        let joint = JointBuilder::spherical()
            .anchor1(Vec3::new(0.0, 1.0, 0.0))
            .anchor2(Vec3::new(0.0, -1.0, 0.0))
            .build();

        match joint {
            Joint::Spherical(config) => {
                assert_eq!(config.anchor1, Vec3::new(0.0, 1.0, 0.0));
                assert_eq!(config.anchor2, Vec3::new(0.0, -1.0, 0.0));
            }
            _ => panic!("Expected Spherical joint"),
        }
    }

    #[test]
    fn test_joint_motor() {
        let motor = JointMotor::new(10.0)
            .with_max_force(100.0)
            .with_stiffness(0.5)
            .with_damping(0.1);

        assert_eq!(motor.target_velocity, 10.0);
        assert_eq!(motor.max_force, 100.0);
        assert_eq!(motor.stiffness, 0.5);
        assert_eq!(motor.damping, 0.1);
    }

    #[test]
    fn test_revolute_with_motor() {
        let motor = JointMotor::new(5.0).with_max_force(50.0);

        let joint = JointBuilder::revolute().axis(Vec3::Y).motor(motor).build();

        match joint {
            Joint::Revolute(config) => {
                assert!(config.motor.is_some());
                let motor = config.motor.unwrap();
                assert_eq!(motor.target_velocity, 5.0);
                assert_eq!(motor.max_force, 50.0);
            }
            _ => panic!("Expected Revolute joint"),
        }
    }

    #[test]
    fn test_joint_serialization() {
        let joint = JointBuilder::revolute()
            .anchor1(Vec3::new(1.0, 2.0, 3.0))
            .anchor2(Vec3::new(4.0, 5.0, 6.0))
            .axis(Vec3::Y)
            .limits(-1.0, 1.0)
            .build();

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&joint).unwrap();

        // Deserialize back
        let deserialized: Joint = serde_yaml::from_str(&yaml).unwrap();

        match (joint, deserialized) {
            (Joint::Revolute(orig), Joint::Revolute(deser)) => {
                assert_eq!(orig.anchor1, deser.anchor1);
                assert_eq!(orig.anchor2, deser.anchor2);
                assert_eq!(orig.axis, deser.axis);
                assert_eq!(orig.limits, deser.limits);
            }
            _ => panic!("Serialization failed"),
        }
    }
}
