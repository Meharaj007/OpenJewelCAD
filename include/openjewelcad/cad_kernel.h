#ifndef OPENJEWELCAD_CAD_KERNEL_H
#define OPENJEWELCAD_CAD_KERNEL_H

#include <stdint.h>

#if defined(_WIN32) && defined(OPENJEWELCAD_KERNEL_BUILD)
#  define OJCAD_API __declspec(dllexport)
#elif defined(_WIN32)
#  define OJCAD_API __declspec(dllimport)
#else
#  define OJCAD_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OJCAD_Shape OJCAD_Shape;
typedef struct OJCAD_Mesh OJCAD_Mesh;

typedef enum OJCAD_Status {
    OJCAD_STATUS_OK = 0,
    OJCAD_STATUS_INVALID_ARGUMENT = 1,
    OJCAD_STATUS_INVALID_SHAPE = 2,
    OJCAD_STATUS_KERNEL_FAILURE = 3
} OJCAD_Status;

typedef enum OJCAD_BooleanOperation {
    OJCAD_BOOLEAN_UNION = 1,
    OJCAD_BOOLEAN_DIFFERENCE = 2,
    OJCAD_BOOLEAN_INTERSECTION = 3
} OJCAD_BooleanOperation;

typedef struct OJCAD_Error {
    OJCAD_Status code;
    char message[256];
} OJCAD_Error;

/// Return the C ABI version supported by this library.
OJCAD_API uint32_t ojcad_abi_version(void);

/// Create an axis-aligned box with positive dimensions in millimetres.
OJCAD_API OJCAD_Status ojcad_box_create(double x_mm, double y_mm, double z_mm,
                                         OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a cylinder along the Z axis with positive radius and height in millimetres.
OJCAD_API OJCAD_Status ojcad_cylinder_create(double radius_mm, double height_mm,
                                              OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a cone along the Z axis with a positive base radius and height in millimetres.
OJCAD_API OJCAD_Status ojcad_cone_create(double radius_mm, double height_mm,
                                          OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a conical frustum along Z with distinct positive base/top radii and height in millimetres.
OJCAD_API OJCAD_Status ojcad_truncated_cone_create(double base_radius_mm, double top_radius_mm,
                                                    double height_mm, OJCAD_Shape** out_shape,
                                                    OJCAD_Error* out_error);
/// Create an ellipsoid centered at the origin with positive X/Y/Z radii in millimetres.
OJCAD_API OJCAD_Status ojcad_ellipsoid_create(double radius_x_mm, double radius_y_mm,
                                               double radius_z_mm, OJCAD_Shape** out_shape,
                                               OJCAD_Error* out_error);
/// Create a straight hollow pipe along Z with outer/inner radii and length in millimetres.
OJCAD_API OJCAD_Status ojcad_pipe_create(double outer_radius_mm, double inner_radius_mm,
                                          double length_mm, OJCAD_Shape** out_shape,
                                          OJCAD_Error* out_error);
/// Create a sphere centered at the origin with positive radius in millimetres.
OJCAD_API OJCAD_Status ojcad_sphere_create(double radius_mm,
                                            OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a ring-shaped torus with major radius greater than tube radius, in millimetres.
OJCAD_API OJCAD_Status ojcad_torus_create(double major_radius_mm, double tube_radius_mm,
                                           OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a faceted round brilliant stone with a finite culet, in millimetres.
OJCAD_API OJCAD_Status ojcad_brilliant_gem_create(double diameter_mm, double total_height_mm,
                                                   double table_diameter_mm,
                                                   OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a closed circular wire centered at the origin in XY, with radius in millimetres.
OJCAD_API OJCAD_Status ojcad_circle_curve_create(double radius_mm,
                                                  OJCAD_Shape** out_shape,
                                                  OJCAD_Error* out_error);
/// Create a planar circular face centered at the origin in XY, with radius in millimetres.
OJCAD_API OJCAD_Status ojcad_planar_circle_surface_create(double radius_mm,
                                                           OJCAD_Shape** out_shape,
                                                           OJCAD_Error* out_error);
/// Extrude a circular XY profile along +Z, with radius and height in millimetres.
OJCAD_API OJCAD_Status ojcad_extruded_circle_create(double radius_mm, double height_mm,
                                                      OJCAD_Shape** out_shape,
                                                      OJCAD_Error* out_error);
/// Extrude a circular profile by height along a 3D direction, normalized internally.
/// The vector must include a non-zero Z component to create a solid.
OJCAD_API OJCAD_Status ojcad_extruded_circle_vector_create(
    double radius_mm, double height_mm, double direction_x, double direction_y,
    double direction_z, OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a closed analytic ellipse wire centered at the origin in XY.
OJCAD_API OJCAD_Status ojcad_ellipse_curve_create(double radius_x_mm, double radius_y_mm,
                                                   OJCAD_Shape** out_shape,
                                                   OJCAD_Error* out_error);
/// Create a planar analytic ellipse face centered at the origin in XY.
OJCAD_API OJCAD_Status ojcad_planar_ellipse_surface_create(double radius_x_mm,
                                                            double radius_y_mm,
                                                            OJCAD_Shape** out_shape,
                                                            OJCAD_Error* out_error);
/// Extrude an analytic ellipse profile along +Z.
OJCAD_API OJCAD_Status ojcad_extruded_ellipse_create(double radius_x_mm, double radius_y_mm,
                                                     double height_mm,
                                                     OJCAD_Shape** out_shape,
                                                     OJCAD_Error* out_error);
/// Extrude an elliptical profile by height along a 3D direction, normalized internally.
/// The vector must include a non-zero Z component to create a solid.
OJCAD_API OJCAD_Status ojcad_extruded_ellipse_vector_create(
    double radius_x_mm, double radius_y_mm, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create an open circular arc in XY. Start angle is measured CCW from +X; sweep is positive.
OJCAD_API OJCAD_Status ojcad_arc_curve_create(double radius_mm, double start_angle_deg,
                                               double sweep_angle_deg,
                                               OJCAD_Shape** out_shape,
                                               OJCAD_Error* out_error);
/// Extrude an open circular arc along +Z to create an exact cylindrical surface shell.
OJCAD_API OJCAD_Status ojcad_extruded_arc_surface_create(
    double radius_mm, double start_angle_deg, double sweep_angle_deg, double height_mm,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Extrude an open circular arc by height in a normalized 3D direction.
OJCAD_API OJCAD_Status ojcad_extruded_arc_surface_vector_create(
    double radius_mm, double start_angle_deg, double sweep_angle_deg, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create an open straight wire between two XY points in millimetres.
OJCAD_API OJCAD_Status ojcad_line_curve_create(double start_x_mm, double start_y_mm,
                                                double end_x_mm, double end_y_mm,
                                                OJCAD_Shape** out_shape,
                                                OJCAD_Error* out_error);
/// Create a closed polyline curve from consecutive XY points in millimetres.
OJCAD_API OJCAD_Status ojcad_closed_polyline_create(const double* points_xy_mm,
                                                     uint32_t point_count,
                                                     OJCAD_Shape** out_shape,
                                                     OJCAD_Error* out_error);
/// Create an open polyline wire from 2..=64 consecutive XY points in millimetres.
OJCAD_API OJCAD_Status ojcad_open_polyline_curve_create(const double* points_xy_mm,
                                                         uint32_t point_count,
                                                         OJCAD_Shape** out_shape,
                                                         OJCAD_Error* out_error);
/// Create an open cubic Bezier curve in XY from exactly four control points.
OJCAD_API OJCAD_Status ojcad_bezier_curve_create(const double* control_points_xy_mm,
                                                  uint32_t point_count,
                                                  OJCAD_Shape** out_shape,
                                                  OJCAD_Error* out_error);
/// Extrude an open cubic Bezier curve into a surface along a supplied 3D vector.
OJCAD_API OJCAD_Status ojcad_extruded_bezier_surface_vector_create(
    const double* control_points_xy_mm, uint32_t point_count, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Extrude an open XY line or polyline along +Z to create a surface shell.
OJCAD_API OJCAD_Status ojcad_extruded_open_polyline_surface_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Extrude an open XY line or polyline by height in a normalized 3D direction.
OJCAD_API OJCAD_Status ojcad_extruded_open_polyline_surface_vector_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Revolve an open radius/height profile around +Y into a 360-degree surface shell.
OJCAD_API OJCAD_Status ojcad_revolved_open_polyline_surface_create(
    const double* points_xy_mm, uint32_t point_count,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a planar face from a simple closed XY polygon in millimetres.
OJCAD_API OJCAD_Status ojcad_planar_polygon_surface_create(const double* points_xy_mm,
                                                            uint32_t point_count,
                                                            OJCAD_Shape** out_shape,
                                                            OJCAD_Error* out_error);
/// Extrude a simple closed XY polygon along +Z. Points are consecutive XY pairs in millimetres.
OJCAD_API OJCAD_Status ojcad_extruded_polygon_create(const double* points_xy_mm,
                                                      uint32_t point_count,
                                                      double height_mm,
                                                      OJCAD_Shape** out_shape,
                                                      OJCAD_Error* out_error);
/// Extrude a closed polygon by height along a 3D direction, normalized internally.
/// The vector must include a non-zero Z component to create a solid.
OJCAD_API OJCAD_Status ojcad_extruded_polygon_vector_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Revolve a simple XY polygon profile around the Y axis to make a 360-degree solid.
/// Every profile point must have a positive X coordinate, which represents radius in millimetres.
OJCAD_API OJCAD_Status ojcad_revolved_polygon_create(const double* points_xy_mm,
                                                       uint32_t point_count,
                                                       OJCAD_Shape** out_shape,
                                                       OJCAD_Error* out_error);
OJCAD_API OJCAD_Status ojcad_shape_volume_mm3(const OJCAD_Shape* shape,
                                               double* out_volume, OJCAD_Error* out_error);
OJCAD_API OJCAD_Status ojcad_shape_is_valid(const OJCAD_Shape* shape,
                                             int32_t* out_valid, OJCAD_Error* out_error);
/// Return [xmin, ymin, zmin, xmax, ymax, zmax] bounds in millimetres.
OJCAD_API OJCAD_Status ojcad_shape_bounds_mm(const OJCAD_Shape* shape,
                                              double out_bounds[6], OJCAD_Error* out_error);
/// Return [volume mm^3, area mm^2, center X mm, center Y mm, center Z mm].
OJCAD_API OJCAD_Status ojcad_shape_mass_properties(const OJCAD_Shape* shape,
                                                   double out_properties[5],
                                                   OJCAD_Error* out_error);
/// Return total edge length in millimetres.
OJCAD_API OJCAD_Status ojcad_shape_length_mm(const OJCAD_Shape* shape,
                                              double* out_length,
                                              OJCAD_Error* out_error);
/// Return a transformed copy of a shape. Rotations are applied X, then Y, then Z, in degrees.
OJCAD_API OJCAD_Status ojcad_shape_transform_mm_degrees(
    const OJCAD_Shape* shape, double translate_x_mm, double translate_y_mm,
    double translate_z_mm, double rotate_x_deg, double rotate_y_deg,
    double rotate_z_deg, double uniform_scale, OJCAD_Shape** out_shape,
    OJCAD_Error* out_error);
/// Mirror a shape across world-origin planes. Bits 0, 1, and 2 select YZ, XZ, and XY.
OJCAD_API OJCAD_Status ojcad_shape_mirror(const OJCAD_Shape* shape,
                                           uint32_t plane_mask,
                                           OJCAD_Shape** out_shape,
                                           OJCAD_Error* out_error);
/// Apply a B-rep union, difference (base minus tool), or intersection.
OJCAD_API OJCAD_Status ojcad_shape_boolean(const OJCAD_Shape* base,
                                            const OJCAD_Shape* tool,
                                            OJCAD_BooleanOperation operation,
                                            double tolerance_mm,
                                            OJCAD_Shape** out_shape,
                                            OJCAD_Error* out_error);
/// Duplicate a shape as a linear pattern. Copies includes the original; 2..=64 is supported.
OJCAD_API OJCAD_Status ojcad_shape_linear_array(const OJCAD_Shape* shape, uint32_t copies,
                                                 double step_x_mm, double step_y_mm,
                                                 double step_z_mm, OJCAD_Shape** out_shape,
                                                 OJCAD_Error* out_error);
/// Duplicate a shape around the world Z axis. Angle is the increment in degrees per copy.
OJCAD_API OJCAD_Status ojcad_shape_polar_array_z(const OJCAD_Shape* shape, uint32_t copies,
                                                  double angle_step_deg,
                                                  OJCAD_Shape** out_shape,
                                                  OJCAD_Error* out_error);
/// Tessellate a shape. Deflection is in mm; angular deflection is in radians.
OJCAD_API OJCAD_Status ojcad_shape_tessellate(const OJCAD_Shape* shape,
                                               double linear_deflection_mm,
                                               double angular_deflection_radians,
                                               OJCAD_Mesh** out_mesh,
                                               OJCAD_Error* out_error);
/// Return vertex and triangle counts for a tessellated mesh.
OJCAD_API OJCAD_Status ojcad_mesh_counts(const OJCAD_Mesh* mesh,
                                          uint64_t* out_vertex_count,
                                          uint64_t* out_triangle_count,
                                          OJCAD_Error* out_error);
/// Copy xyz doubles and triangle vertex indices into caller-owned buffers.
OJCAD_API OJCAD_Status ojcad_mesh_copy(const OJCAD_Mesh* mesh,
                                        double* out_vertices_xyz,
                                        uint64_t vertex_capacity,
                                        uint32_t* out_triangle_indices,
                                        uint64_t triangle_capacity,
                                        OJCAD_Error* out_error);
OJCAD_API void ojcad_mesh_destroy(OJCAD_Mesh* mesh);
OJCAD_API void ojcad_shape_destroy(OJCAD_Shape* shape);

#ifdef __cplusplus
}
#endif

#endif
