#include "openjewelcad/cad_kernel.h"

#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS_Shape.hxx>

#include <cmath>
#include <cstring>
#include <exception>
#include <new>

struct OJCAD_Shape {
    TopoDS_Shape value;
};

namespace {
void set_error(OJCAD_Error* error, OJCAD_Status code, const char* message) noexcept {
    if (error == nullptr) return;
    error->code = code;
    std::strncpy(error->message, message, sizeof(error->message) - 1);
    error->message[sizeof(error->message) - 1] = '\0';
}

void clear_error(OJCAD_Error* error) noexcept {
    if (error != nullptr) set_error(error, OJCAD_STATUS_OK, "");
}

template <typename Factory>
OJCAD_Status create_shape(Factory&& factory, OJCAD_Shape** out_shape,
                          OJCAD_Error* out_error) noexcept {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    try {
        TopoDS_Shape value = factory();
        if (value.IsNull()) {
            set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "OCCT could not construct the shape");
            return OJCAD_STATUS_KERNEL_FAILURE;
        }
        OJCAD_Shape* result = new (std::nothrow) OJCAD_Shape{value};
        if (result == nullptr) {
            set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "could not allocate shape handle");
            return OJCAD_STATUS_KERNEL_FAILURE;
        }
        *out_shape = result;
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}
}

extern "C" OJCAD_Status ojcad_box_create(double x_mm, double y_mm, double z_mm,
                                           OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(x_mm) || !std::isfinite(y_mm) ||
        !std::isfinite(z_mm) || x_mm <= 0.0 || y_mm <= 0.0 || z_mm <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "box dimensions must be finite, positive millimetre values");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeBox builder(x_mm, y_mm, z_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_cylinder_create(double radius_mm, double height_mm,
                                               OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_mm) || !std::isfinite(height_mm) ||
        radius_mm <= 0.0 || height_mm <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "cylinder radius and height must be finite, positive millimetre values");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeCylinder builder(radius_mm, height_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_sphere_create(double radius_mm,
                                             OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_mm) || radius_mm <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "sphere radius must be a finite, positive millimetre value");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeSphere builder(radius_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_torus_create(double major_radius_mm, double tube_radius_mm,
                                            OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(major_radius_mm) || !std::isfinite(tube_radius_mm) ||
        tube_radius_mm <= 0.0 || major_radius_mm <= tube_radius_mm) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "torus major radius must be greater than its positive tube radius");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeTorus builder(major_radius_mm, tube_radius_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" uint32_t ojcad_abi_version(void) { return 2; }

extern "C" OJCAD_Status ojcad_shape_volume_mm3(const OJCAD_Shape* shape,
                                                 double* out_volume, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_volume == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "shape and output pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        GProp_GProps properties;
        BRepGProp::VolumeProperties(shape->value, properties);
        const double volume = properties.Mass();
        if (!std::isfinite(volume) || volume <= 0.0) {
            set_error(out_error, OJCAD_STATUS_INVALID_SHAPE, "shape has no finite positive volume");
            return OJCAD_STATUS_INVALID_SHAPE;
        }
        *out_volume = volume;
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" OJCAD_Status ojcad_shape_is_valid(const OJCAD_Shape* shape,
                                               int32_t* out_valid, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_valid == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "shape and output pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        *out_valid = BRepCheck_Analyzer(shape->value).IsValid() ? 1 : 0;
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" void ojcad_shape_destroy(OJCAD_Shape* shape) { delete shape; }
