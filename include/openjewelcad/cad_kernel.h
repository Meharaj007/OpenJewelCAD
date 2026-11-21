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

typedef enum OJCAD_Status {
    OJCAD_STATUS_OK = 0,
    OJCAD_STATUS_INVALID_ARGUMENT = 1,
    OJCAD_STATUS_INVALID_SHAPE = 2,
    OJCAD_STATUS_KERNEL_FAILURE = 3
} OJCAD_Status;

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
/// Create a sphere centered at the origin with positive radius in millimetres.
OJCAD_API OJCAD_Status ojcad_sphere_create(double radius_mm,
                                            OJCAD_Shape** out_shape, OJCAD_Error* out_error);
/// Create a ring-shaped torus with major radius greater than tube radius, in millimetres.
OJCAD_API OJCAD_Status ojcad_torus_create(double major_radius_mm, double tube_radius_mm,
                                           OJCAD_Shape** out_shape, OJCAD_Error* out_error);
OJCAD_API OJCAD_Status ojcad_shape_volume_mm3(const OJCAD_Shape* shape,
                                               double* out_volume, OJCAD_Error* out_error);
OJCAD_API OJCAD_Status ojcad_shape_is_valid(const OJCAD_Shape* shape,
                                             int32_t* out_valid, OJCAD_Error* out_error);
OJCAD_API void ojcad_shape_destroy(OJCAD_Shape* shape);

#ifdef __cplusplus
}
#endif

#endif
