#include "openjewelcad/cad_kernel.h"

#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepBndLib.hxx>
#include <BRep_Builder.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <BRepBuilderAPI_GTransform.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <GProp_GProps.hxx>
#include <GCPnts_AbscissaPoint.hxx>
#include <GeomAdaptor_Curve.hxx>
#include <Bnd_Box.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Circ.hxx>
#include <gp_Elips.hxx>
#include <gp_Dir.hxx>
#include <gp_GTrsf.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>
#include <NCollection_List.hxx>
#include <Poly_Triangulation.hxx>
#include <TopAbs_Orientation.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp_Explorer.hxx>
#include <TopLoc_Location.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Compound.hxx>
#include <TopoDS_Shell.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Wire.hxx>

#include <cmath>
#include <algorithm>
#include <cstdint>
#include <cstring>
#include <exception>
#include <memory>
#include <new>
#include <array>
#include <vector>

struct OJCAD_Shape {
    TopoDS_Shape value;
};

struct OJCAD_Mesh {
    std::vector<double> vertices_xyz;
    std::vector<uint32_t> triangle_indices;
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

TopoDS_Face polygon_face(const std::vector<gp_Pnt>& points) {
    BRepBuilderAPI_MakePolygon polygon;
    for (const gp_Pnt& point : points) polygon.Add(point);
    polygon.Close();
    BRepBuilderAPI_MakeFace face(polygon.Wire());
    if (!face.IsDone()) return TopoDS_Face{};
    return face.Face();
}

TopoDS_Wire circle_wire(double radius_mm) {
    const gp_Circ circle(gp_Ax2(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0)), radius_mm);
    BRepBuilderAPI_MakeEdge edge(circle);
    if (!edge.IsDone()) return TopoDS_Wire{};
    BRepBuilderAPI_MakeWire wire(edge.Edge());
    if (!wire.IsDone()) return TopoDS_Wire{};
    return wire.Wire();
}

TopoDS_Wire ellipse_wire(double radius_x_mm, double radius_y_mm) {
    if (std::abs(radius_x_mm - radius_y_mm) <= 1e-12 * std::max(radius_x_mm, radius_y_mm)) {
        return circle_wire((radius_x_mm + radius_y_mm) * 0.5);
    }
    const bool major_is_y = radius_y_mm > radius_x_mm;
    const gp_Dir x_direction = major_is_y ? gp_Dir(0.0, 1.0, 0.0)
                                          : gp_Dir(1.0, 0.0, 0.0);
    const gp_Ax2 axes(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0), x_direction);
    const gp_Elips ellipse(axes, std::max(radius_x_mm, radius_y_mm),
                          std::min(radius_x_mm, radius_y_mm));
    BRepBuilderAPI_MakeEdge edge(ellipse);
    if (!edge.IsDone()) return TopoDS_Wire{};
    BRepBuilderAPI_MakeWire wire(edge.Edge());
    if (!wire.IsDone()) return TopoDS_Wire{};
    return wire.Wire();
}

bool make_extrusion_displacement(double height_mm, double direction_x, double direction_y,
                                double direction_z, gp_Vec& displacement) {
    const double direction_length = std::hypot(std::hypot(direction_x, direction_y), direction_z);
    if (!std::isfinite(height_mm) || height_mm <= 0.0 || height_mm > 1e7 ||
        !std::isfinite(direction_x) || !std::isfinite(direction_y) ||
        !std::isfinite(direction_z) || !std::isfinite(direction_length) ||
        direction_length <= 1e-12) {
        return false;
    }
    displacement = gp_Vec(direction_x, direction_y, direction_z) * (height_mm / direction_length);
    return true;
}

bool make_solid_extrusion_displacement(double height_mm, double direction_x, double direction_y,
                                       double direction_z, gp_Vec& displacement) {
    const double direction_length = std::hypot(std::hypot(direction_x, direction_y), direction_z);
    return make_extrusion_displacement(height_mm, direction_x, direction_y, direction_z,
                                       displacement) &&
           std::abs(direction_z) > direction_length * 1e-12;
}

TopoDS_Shape make_round_brilliant(double diameter_mm, double total_height_mm,
                                 double table_diameter_mm) {
    static constexpr int segments = 32;
    const double radius = diameter_mm * 0.5;
    const double table_radius = table_diameter_mm * 0.5;
    auto make_ring = [](double ring_radius, double z) {
        std::array<gp_Pnt, segments> ring;
        for (int index = 0; index < segments; ++index) {
            const double angle = 2.0 * std::acos(-1.0) * index / segments;
            ring[index] = gp_Pnt(ring_radius * std::cos(angle),
                                 ring_radius * std::sin(angle), z);
        }
        return ring;
    };

    const auto culet = make_ring(radius * 0.06, 0.0);
    const auto pavilion = make_ring(radius * 0.90, total_height_mm * 0.43);
    const auto girdle = make_ring(radius, total_height_mm * 0.49);
    const auto star = make_ring((radius + table_radius) * 0.5,
                                total_height_mm * 0.77);
    const auto table = make_ring(table_radius, total_height_mm);

    BRepBuilderAPI_Sewing sewing(1e-7);
    auto add_cap = [&sewing](const auto& ring, bool reverse) {
        std::vector<gp_Pnt> points(ring.begin(), ring.end());
        TopoDS_Face face = polygon_face(points);
        if (face.IsNull()) return false;
        if (reverse) face.Reverse();
        sewing.Add(face);
        return true;
    };
    auto add_faceted_band = [&sewing](const auto& lower, const auto& upper) {
        for (int index = 0; index < segments; ++index) {
            const int next = (index + 1) % segments;
            TopoDS_Face first = polygon_face({lower[index], lower[next], upper[next]});
            TopoDS_Face second = polygon_face({lower[index], upper[next], upper[index]});
            if (first.IsNull() || second.IsNull()) return false;
            sewing.Add(first);
            sewing.Add(second);
        }
        return true;
    };

    if (!add_cap(culet, true) || !add_cap(table, false) ||
        !add_faceted_band(culet, pavilion) || !add_faceted_band(pavilion, girdle) ||
        !add_faceted_band(girdle, star) || !add_faceted_band(star, table)) {
        return TopoDS_Shape{};
    }

    sewing.Perform();
    const TopoDS_Shape sewed = sewing.SewedShape();
    TopExp_Explorer shells(sewed, TopAbs_SHELL);
    if (!shells.More()) return TopoDS_Shape{};
    BRepBuilderAPI_MakeSolid solid_builder(TopoDS::Shell(shells.Current()));
    if (!solid_builder.IsDone()) return TopoDS_Shape{};
    TopoDS_Solid solid = solid_builder.Solid();
    if (!BRepLib::OrientClosedSolid(solid)) return TopoDS_Shape{};
    return solid;
}

double cross_2d(const gp_Pnt& a, const gp_Pnt& b, const gp_Pnt& c) {
    return (b.X() - a.X()) * (c.Y() - a.Y()) - (b.Y() - a.Y()) * (c.X() - a.X());
}

bool point_on_segment_2d(const gp_Pnt& point, const gp_Pnt& a, const gp_Pnt& b) {
    constexpr double epsilon = 1e-9;
    return std::abs(cross_2d(a, b, point)) <= epsilon &&
           point.X() >= std::min(a.X(), b.X()) - epsilon &&
           point.X() <= std::max(a.X(), b.X()) + epsilon &&
           point.Y() >= std::min(a.Y(), b.Y()) - epsilon &&
           point.Y() <= std::max(a.Y(), b.Y()) + epsilon;
}

bool segments_intersect_2d(const gp_Pnt& a, const gp_Pnt& b,
                           const gp_Pnt& c, const gp_Pnt& d) {
    constexpr double epsilon = 1e-9;
    const double ab_c = cross_2d(a, b, c);
    const double ab_d = cross_2d(a, b, d);
    const double cd_a = cross_2d(c, d, a);
    const double cd_b = cross_2d(c, d, b);
    const auto sign = [epsilon](double value) {
        return value > epsilon ? 1 : (value < -epsilon ? -1 : 0);
    };
    const int first = sign(ab_c);
    const int second = sign(ab_d);
    const int third = sign(cd_a);
    const int fourth = sign(cd_b);
    if (first != second && third != fourth) return true;
    return (first == 0 && point_on_segment_2d(c, a, b)) ||
           (second == 0 && point_on_segment_2d(d, a, b)) ||
           (third == 0 && point_on_segment_2d(a, c, d)) ||
           (fourth == 0 && point_on_segment_2d(b, c, d));
}

bool polygon_is_simple(const std::vector<gp_Pnt>& points) {
    const size_t count = points.size();
    for (size_t first = 0; first < count; ++first) {
        const size_t first_next = (first + 1) % count;
        for (size_t second = first + 1; second < count; ++second) {
            const size_t second_next = (second + 1) % count;
            if (first == second || first_next == second || second_next == first) continue;
            if (segments_intersect_2d(points[first], points[first_next],
                                      points[second], points[second_next])) {
                return false;
            }
        }
    }
    return true;
}

bool valid_xy_profile(const std::vector<gp_Pnt>& points) {
    if (points.size() < 3 || points.size() > 64) return false;
    double twice_area = 0.0;
    for (size_t index = 0; index < points.size(); ++index) {
        const gp_Pnt& current = points[index];
        const gp_Pnt& next = points[(index + 1) % points.size()];
        if (!std::isfinite(current.X()) || !std::isfinite(current.Y()) ||
            std::abs(current.X()) > 1e7 || std::abs(current.Y()) > 1e7) {
            return false;
        }
        const double dx = next.X() - current.X();
        const double dy = next.Y() - current.Y();
        if (dx * dx + dy * dy <= 1e-16) return false;
        twice_area += current.X() * next.Y() - next.X() * current.Y();
    }
    return std::isfinite(twice_area) && std::abs(twice_area) > 1e-12 &&
           polygon_is_simple(points);
}

std::vector<gp_Pnt> read_xy_points(const double* coordinates, uint32_t point_count) {
    std::vector<gp_Pnt> points;
    points.reserve(point_count);
    for (uint32_t index = 0; index < point_count; ++index) {
        points.emplace_back(coordinates[index * 2], coordinates[index * 2 + 1], 0.0);
    }
    return points;
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

extern "C" OJCAD_Status ojcad_cone_create(double radius_mm, double height_mm,
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
                  "cone radius and height must be finite, positive millimetre values");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeCone builder(radius_mm, 0.0, height_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_truncated_cone_create(double base_radius_mm, double top_radius_mm,
                                                      double height_mm, OJCAD_Shape** out_shape,
                                                      OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(base_radius_mm) || !std::isfinite(top_radius_mm) ||
        !std::isfinite(height_mm) || base_radius_mm <= 0.0 || top_radius_mm <= 0.0 ||
        height_mm <= 0.0 || base_radius_mm == top_radius_mm) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "truncated cone requires distinct positive base/top radii and positive height");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeCone builder(base_radius_mm, top_radius_mm, height_mm);
        builder.Build();
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_ellipsoid_create(double radius_x_mm, double radius_y_mm,
                                                 double radius_z_mm, OJCAD_Shape** out_shape,
                                                 OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_x_mm) || !std::isfinite(radius_y_mm) ||
        !std::isfinite(radius_z_mm) || radius_x_mm <= 0.0 ||
        radius_y_mm <= 0.0 || radius_z_mm <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "ellipsoid X/Y/Z radii must be finite, positive millimetre values");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeSphere sphere(1.0);
        sphere.Build();
        if (!sphere.IsDone()) return TopoDS_Shape{};

        gp_GTrsf scaling;
        scaling.SetValue(1, 1, radius_x_mm);
        scaling.SetValue(2, 2, radius_y_mm);
        scaling.SetValue(3, 3, radius_z_mm);
        BRepBuilderAPI_GTransform builder(sphere.Shape(), scaling, true);
        return builder.IsDone() ? builder.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_pipe_create(double outer_radius_mm, double inner_radius_mm,
                                            double length_mm, OJCAD_Shape** out_shape,
                                            OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(outer_radius_mm) || !std::isfinite(inner_radius_mm) ||
        !std::isfinite(length_mm) || outer_radius_mm <= 0.0 || inner_radius_mm <= 0.0 ||
        length_mm <= 0.0 || inner_radius_mm >= outer_radius_mm) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "pipe requires positive radii and length, with inner radius below outer radius");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepPrimAPI_MakeCylinder outer_builder(outer_radius_mm, length_mm);
        BRepPrimAPI_MakeCylinder inner_builder(inner_radius_mm, length_mm);
        outer_builder.Build();
        inner_builder.Build();
        if (!outer_builder.IsDone() || !inner_builder.IsDone()) return TopoDS_Shape{};

        NCollection_List<TopoDS_Shape> arguments;
        arguments.Append(outer_builder.Shape());
        NCollection_List<TopoDS_Shape> tools;
        tools.Append(inner_builder.Shape());
        BRepAlgoAPI_Cut cutter;
        cutter.SetArguments(arguments);
        cutter.SetTools(tools);
        cutter.Build();
        return cutter.IsDone() ? cutter.Shape() : TopoDS_Shape{};
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

extern "C" OJCAD_Status ojcad_brilliant_gem_create(double diameter_mm, double total_height_mm,
                                                     double table_diameter_mm,
                                                     OJCAD_Shape** out_shape,
                                                     OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(diameter_mm) || !std::isfinite(total_height_mm) ||
        !std::isfinite(table_diameter_mm) || diameter_mm <= 0.0 || total_height_mm <= 0.0 ||
        table_diameter_mm <= 0.0 || table_diameter_mm >= diameter_mm) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "round brilliant requires positive height and a table diameter below the girdle diameter");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        return make_round_brilliant(diameter_mm, total_height_mm, table_diameter_mm);
    }, out_shape, out_error);
}

extern "C" uint32_t ojcad_abi_version(void) { return 23; }

extern "C" OJCAD_Status ojcad_circle_curve_create(double radius_mm,
                                                    OJCAD_Shape** out_shape,
                                                    OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 1e7) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "circle radius must be finite and within (0, 10,000,000] mm");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = circle_wire(radius_mm);
        return wire.IsNull() ? TopoDS_Shape{} : TopoDS_Shape(wire);
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_planar_circle_surface_create(double radius_mm,
                                                             OJCAD_Shape** out_shape,
                                                             OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 1e7) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "circle radius must be finite and within (0, 10,000,000] mm");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = circle_wire(radius_mm);
        if (wire.IsNull()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeFace face(wire);
        return face.IsDone() ? TopoDS_Shape(face.Face()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_circle_create(double radius_mm, double height_mm,
                                                      OJCAD_Shape** out_shape,
                                                      OJCAD_Error* out_error) {
    return ojcad_extruded_circle_vector_create(radius_mm, height_mm, 0.0, 0.0, 1.0,
                                              out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_circle_vector_create(
    double radius_mm, double height_mm, double direction_x, double direction_y,
    double direction_z, OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    gp_Vec displacement;
    if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 1e7 ||
        !make_solid_extrusion_displacement(height_mm, direction_x, direction_y, direction_z,
                                           displacement)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "circle solid extrusion needs a finite positive radius and height, and a finite direction with a non-zero Z component");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = circle_wire(radius_mm);
        if (wire.IsNull()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeFace face(wire);
        if (!face.IsDone()) return TopoDS_Shape{};
        BRepPrimAPI_MakePrism prism(face.Face(), displacement);
        prism.Build();
        return prism.IsDone() ? prism.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_ellipse_curve_create(double radius_x_mm, double radius_y_mm,
                                                     OJCAD_Shape** out_shape,
                                                     OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_x_mm) || !std::isfinite(radius_y_mm) ||
        radius_x_mm <= 0.0 || radius_y_mm <= 0.0 || radius_x_mm > 1e7 || radius_y_mm > 1e7) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "ellipse radii must be finite positive values within 10,000,000 mm");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = ellipse_wire(radius_x_mm, radius_y_mm);
        return wire.IsNull() ? TopoDS_Shape{} : TopoDS_Shape(wire);
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_planar_ellipse_surface_create(double radius_x_mm,
                                                              double radius_y_mm,
                                                              OJCAD_Shape** out_shape,
                                                              OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_x_mm) || !std::isfinite(radius_y_mm) ||
        radius_x_mm <= 0.0 || radius_y_mm <= 0.0 || radius_x_mm > 1e7 || radius_y_mm > 1e7) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "ellipse radii must be finite positive values within 10,000,000 mm");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = ellipse_wire(radius_x_mm, radius_y_mm);
        if (wire.IsNull()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeFace face(wire);
        return face.IsDone() ? TopoDS_Shape(face.Face()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_ellipse_create(double radius_x_mm, double radius_y_mm,
                                                       double height_mm,
                                                       OJCAD_Shape** out_shape,
                                                       OJCAD_Error* out_error) {
    return ojcad_extruded_ellipse_vector_create(radius_x_mm, radius_y_mm, height_mm,
                                               0.0, 0.0, 1.0, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_ellipse_vector_create(
    double radius_x_mm, double radius_y_mm, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    gp_Vec displacement;
    if (!std::isfinite(radius_x_mm) || !std::isfinite(radius_y_mm) ||
        radius_x_mm <= 0.0 || radius_y_mm <= 0.0 || radius_x_mm > 1e7 || radius_y_mm > 1e7 ||
        !make_solid_extrusion_displacement(height_mm, direction_x, direction_y, direction_z,
                                           displacement)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "ellipse solid extrusion needs finite positive radii and height, and a finite direction with a non-zero Z component");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const TopoDS_Wire wire = ellipse_wire(radius_x_mm, radius_y_mm);
        if (wire.IsNull()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeFace face(wire);
        if (!face.IsDone()) return TopoDS_Shape{};
        BRepPrimAPI_MakePrism prism(face.Face(), displacement);
        prism.Build();
        return prism.IsDone() ? prism.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_arc_curve_create(double radius_mm, double start_angle_deg,
                                                 double sweep_angle_deg,
                                                 OJCAD_Shape** out_shape,
                                                 OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(radius_mm) || !std::isfinite(start_angle_deg) ||
        !std::isfinite(sweep_angle_deg) || radius_mm <= 0.0 || radius_mm > 1e7 ||
        start_angle_deg < -360.0 || start_angle_deg > 360.0 ||
        sweep_angle_deg <= 0.0 || sweep_angle_deg >= 360.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "arc needs a positive radius, start angle in [-360, 360] degrees, and sweep in (0, 360) degrees");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    const double turn = 2.0 * std::acos(-1.0);
    double start = std::fmod(start_angle_deg, 360.0) * turn / 360.0;
    if (start < 0.0) start += turn;
    const double end = start + sweep_angle_deg * turn / 360.0;
    return create_shape([=]() {
        const gp_Circ circle(gp_Ax2(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0)), radius_mm);
        BRepBuilderAPI_MakeEdge edge(circle, start, end);
        if (!edge.IsDone()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeWire wire(edge.Edge());
        return wire.IsDone() ? TopoDS_Shape(wire.Wire()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_arc_surface_create(
    double radius_mm, double start_angle_deg, double sweep_angle_deg, double height_mm,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    return ojcad_extruded_arc_surface_vector_create(
        radius_mm, start_angle_deg, sweep_angle_deg, height_mm, 0.0, 0.0, 1.0,
        out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_arc_surface_vector_create(
    double radius_mm, double start_angle_deg, double sweep_angle_deg, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    const double direction_length = std::hypot(std::hypot(direction_x, direction_y), direction_z);
    if (!std::isfinite(radius_mm) || !std::isfinite(start_angle_deg) ||
        !std::isfinite(sweep_angle_deg) || !std::isfinite(height_mm) ||
        !std::isfinite(direction_x) || !std::isfinite(direction_y) ||
        !std::isfinite(direction_z) || !std::isfinite(direction_length) || radius_mm <= 0.0 ||
        radius_mm > 1e7 || start_angle_deg < -360.0 || start_angle_deg > 360.0 ||
        sweep_angle_deg <= 0.0 || sweep_angle_deg >= 360.0 || height_mm <= 0.0 ||
        height_mm > 1e7 || direction_length <= 1e-12) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "arc surface needs a positive radius and height, a non-zero finite direction, start in [-360, 360] degrees, and sweep in (0, 360) degrees");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    const double turn = 2.0 * std::acos(-1.0);
    double start = std::fmod(start_angle_deg, 360.0) * turn / 360.0;
    if (start < 0.0) start += turn;
    const double end = start + sweep_angle_deg * turn / 360.0;
    const gp_Vec extrusion(direction_x, direction_y, direction_z);
    const gp_Vec displacement = extrusion * (height_mm / direction_length);
    return create_shape([=]() {
        const gp_Circ circle(gp_Ax2(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0)), radius_mm);
        BRepBuilderAPI_MakeEdge edge(circle, start, end);
        if (!edge.IsDone()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeWire wire(edge.Edge());
        if (!wire.IsDone()) return TopoDS_Shape{};
        BRepPrimAPI_MakePrism prism(wire.Wire(), displacement);
        prism.Build();
        return prism.IsDone() ? prism.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_line_curve_create(double start_x_mm, double start_y_mm,
                                                  double end_x_mm, double end_y_mm,
                                                  OJCAD_Shape** out_shape,
                                                  OJCAD_Error* out_error) {
    clear_error(out_error);
    if (out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "output shape pointer is required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    const std::array<double, 4> coordinates{start_x_mm, start_y_mm, end_x_mm, end_y_mm};
    if (!std::all_of(coordinates.begin(), coordinates.end(), [](double value) {
            return std::isfinite(value) && std::abs(value) <= 1e7;
        }) || std::hypot(end_x_mm - start_x_mm, end_y_mm - start_y_mm) <= 1e-9) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "line endpoints must be finite, within 10,000,000 mm, and distinct");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        BRepBuilderAPI_MakeEdge edge(gp_Pnt(start_x_mm, start_y_mm, 0.0),
                                     gp_Pnt(end_x_mm, end_y_mm, 0.0));
        if (!edge.IsDone()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeWire wire(edge.Edge());
        return wire.IsDone() ? TopoDS_Shape(wire.Wire()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_closed_polyline_create(const double* points_xy_mm,
                                                      uint32_t point_count,
                                                      OJCAD_Shape** out_shape,
                                                      OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polyline points and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (point_count < 3 || point_count > 64) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "closed polyline needs 3..=64 points");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    if (!valid_xy_profile(points)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "closed polyline needs a finite, non-degenerate simple outline");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([points = std::move(points)]() {
        BRepBuilderAPI_MakePolygon wire;
        for (const gp_Pnt& point : points) wire.Add(point);
        wire.Close();
        return wire.IsDone() ? TopoDS_Shape(wire.Wire()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_open_polyline_curve_create(const double* points_xy_mm,
                                                           uint32_t point_count,
                                                           OJCAD_Shape** out_shape,
                                                           OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "open polyline points and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (point_count < 2 || point_count > 64) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "open polyline needs 2..=64 points");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    for (size_t index = 0; index < points.size(); ++index) {
        const gp_Pnt& point = points[index];
        if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) ||
            std::abs(point.X()) > 1e7 || std::abs(point.Y()) > 1e7) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "open polyline coordinates must be finite and within ±10,000,000 mm");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        if (index > 0) {
            const gp_Pnt& previous = points[index - 1];
            const double dx = point.X() - previous.X();
            const double dy = point.Y() - previous.Y();
            if (dx * dx + dy * dy <= 1e-16) {
                set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                          "open polyline segments need distinct consecutive vertices");
                return OJCAD_STATUS_INVALID_ARGUMENT;
            }
        }
    }
    return create_shape([points = std::move(points)]() {
        BRepBuilderAPI_MakePolygon wire;
        for (const gp_Pnt& point : points) wire.Add(point);
        return wire.IsDone() ? TopoDS_Shape(wire.Wire()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_open_polyline_surface_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    return ojcad_extruded_open_polyline_surface_vector_create(
        points_xy_mm, point_count, height_mm, 0.0, 0.0, 1.0, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_open_polyline_surface_vector_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "open curve points and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    const double direction_length = std::hypot(std::hypot(direction_x, direction_y), direction_z);
    if (point_count < 2 || point_count > 64 || !std::isfinite(height_mm) ||
        height_mm <= 0.0 || height_mm > 1e7 || !std::isfinite(direction_x) ||
        !std::isfinite(direction_y) || !std::isfinite(direction_z) ||
        !std::isfinite(direction_length) || direction_length <= 1e-12) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "open-curve extrusion needs 2..=64 points, a finite positive height, and a non-zero finite direction");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    for (size_t index = 0; index < points.size(); ++index) {
        const gp_Pnt& point = points[index];
        if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) ||
            std::abs(point.X()) > 1e7 || std::abs(point.Y()) > 1e7) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "open curve coordinates must be finite and within ±10,000,000 mm");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        if (index > 0) {
            const gp_Pnt& previous = points[index - 1];
            const double dx = point.X() - previous.X();
            const double dy = point.Y() - previous.Y();
            if (dx * dx + dy * dy <= 1e-16) {
                set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                          "open curve segments need distinct consecutive vertices");
                return OJCAD_STATUS_INVALID_ARGUMENT;
            }
        }
    }
    const gp_Vec extrusion(direction_x, direction_y, direction_z);
    const gp_Vec displacement = extrusion * (height_mm / direction_length);
    return create_shape([points = std::move(points), displacement]() {
        BRepBuilderAPI_MakePolygon wire;
        for (const gp_Pnt& point : points) wire.Add(point);
        if (!wire.IsDone()) return TopoDS_Shape{};
        BRepPrimAPI_MakePrism prism(wire.Wire(), displacement);
        prism.Build();
        return prism.IsDone() ? prism.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_revolved_open_polyline_surface_create(
    const double* points_xy_mm, uint32_t point_count,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "open profile points and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (point_count < 2 || point_count > 64) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "revolved surface profile needs 2..=64 points");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    for (size_t index = 0; index < points.size(); ++index) {
        const gp_Pnt& point = points[index];
        if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) ||
            std::abs(point.X()) > 1e7 || std::abs(point.Y()) > 1e7) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "revolved surface profile coordinates must be finite and within ±10,000,000 mm");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        if (point.X() <= 1e-9) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "revolved surface profile radii must stay greater than 0.000000001 mm from the axis");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        if (index > 0) {
            const gp_Pnt& previous = points[index - 1];
            if (previous.SquareDistance(point) <= 1e-16) {
                set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                          "revolved surface profile segments need distinct consecutive vertices");
                return OJCAD_STATUS_INVALID_ARGUMENT;
            }
        }
    }
    return create_shape([points = std::move(points)]() {
        BRepBuilderAPI_MakePolygon profile;
        for (const gp_Pnt& point : points) profile.Add(point);
        if (!profile.IsDone()) return TopoDS_Shape{};
        const gp_Ax1 axis(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 1.0, 0.0));
        BRepPrimAPI_MakeRevol revolution(profile.Wire(), axis, 2.0 * std::acos(-1.0), true);
        revolution.Build();
        return revolution.IsDone() ? revolution.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_planar_polygon_surface_create(const double* points_xy_mm,
                                                             uint32_t point_count,
                                                             OJCAD_Shape** out_shape,
                                                             OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "surface boundary and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (point_count < 3 || point_count > 64) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "planar surface boundary needs 3..=64 points");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    if (!valid_xy_profile(points)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "planar surface needs a finite, non-degenerate simple boundary");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([points = std::move(points)]() {
        const TopoDS_Face face = polygon_face(points);
        return face.IsNull() ? TopoDS_Shape{} : TopoDS_Shape(face);
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_polygon_create(const double* points_xy_mm,
                                                       uint32_t point_count,
                                                       double height_mm,
                                                       OJCAD_Shape** out_shape,
                                                       OJCAD_Error* out_error) {
    return ojcad_extruded_polygon_vector_create(points_xy_mm, point_count, height_mm,
                                                0.0, 0.0, 1.0, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_extruded_polygon_vector_create(
    const double* points_xy_mm, uint32_t point_count, double height_mm,
    double direction_x, double direction_y, double direction_z,
    OJCAD_Shape** out_shape, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polygon points and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    gp_Vec displacement;
    if (point_count < 3 || point_count > 64 ||
        !make_solid_extrusion_displacement(height_mm, direction_x, direction_y, direction_z,
                                           displacement)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polygon solid extrusion needs 3..=64 points, a finite positive height, and a finite direction with a non-zero Z component");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }

    std::vector<gp_Pnt> points;
    points.reserve(point_count);
    double twice_area = 0.0;
    for (uint32_t index = 0; index < point_count; ++index) {
        const double x = points_xy_mm[index * 2];
        const double y = points_xy_mm[index * 2 + 1];
        if (!std::isfinite(x) || !std::isfinite(y) || std::abs(x) > 1e7 || std::abs(y) > 1e7) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "polygon coordinates must be finite millimetre values within ±10,000,000");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        points.emplace_back(x, y, 0.0);
    }
    for (uint32_t index = 0; index < point_count; ++index) {
        const gp_Pnt& current = points[index];
        const gp_Pnt& next = points[(index + 1) % point_count];
        const double dx = next.X() - current.X();
        const double dy = next.Y() - current.Y();
        if (dx * dx + dy * dy <= 1e-16) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "polygon edges must have distinct consecutive vertices");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
        twice_area += current.X() * next.Y() - next.X() * current.Y();
    }
    if (!std::isfinite(twice_area) || std::abs(twice_area) <= 1e-12) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polygon profile must enclose a non-zero area");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    if (!polygon_is_simple(points)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polygon profile must not cross or touch itself");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }

    return create_shape([points = std::move(points), displacement]() {
        BRepBuilderAPI_MakePolygon wire_builder;
        for (const gp_Pnt& point : points) wire_builder.Add(point);
        wire_builder.Close();
        if (!wire_builder.IsDone()) return TopoDS_Shape{};
        BRepBuilderAPI_MakeFace face_builder(wire_builder.Wire());
        if (!face_builder.IsDone()) return TopoDS_Shape{};
        BRepPrimAPI_MakePrism prism(face_builder.Face(), displacement);
        prism.Build();
        return prism.IsDone() ? prism.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_revolved_polygon_create(const double* points_xy_mm,
                                                       uint32_t point_count,
                                                       OJCAD_Shape** out_shape,
                                                       OJCAD_Error* out_error) {
    clear_error(out_error);
    if (points_xy_mm == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "revolve profile and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (point_count < 3 || point_count > 64) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "revolve profile needs 3..=64 points");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    auto points = read_xy_points(points_xy_mm, point_count);
    if (!valid_xy_profile(points)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "revolve profile must be finite, non-degenerate, and simple");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    if (std::any_of(points.begin(), points.end(), [](const gp_Pnt& point) {
            return point.X() <= 1e-9;
        })) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "revolve profile must remain at a positive radius from the Y axis");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([points = std::move(points)]() {
        const TopoDS_Face face = polygon_face(points);
        if (face.IsNull()) return TopoDS_Shape{};
        const gp_Ax1 axis(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 1.0, 0.0));
        BRepPrimAPI_MakeRevol revolution(face, axis, 2.0 * std::acos(-1.0), true);
        revolution.Build();
        return revolution.IsDone() ? revolution.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_shape_volume_mm3(const OJCAD_Shape* shape,
                                                 double* out_volume, OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_volume == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT, "shape and output pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        GProp_GProps properties;
        BRepGProp::VolumeProperties(shape->value, properties, 1e-9);
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

extern "C" OJCAD_Status ojcad_shape_bounds_mm(const OJCAD_Shape* shape,
                                               double out_bounds[6], OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_bounds == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and six-value bounds output are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        Bnd_Box bounds;
        BRepBndLib::AddOptimal(shape->value, bounds, false, false);
        if (bounds.IsVoid() || bounds.IsOpenXmin() || bounds.IsOpenXmax() ||
            bounds.IsOpenYmin() || bounds.IsOpenYmax() || bounds.IsOpenZmin() ||
            bounds.IsOpenZmax()) {
            set_error(out_error, OJCAD_STATUS_INVALID_SHAPE, "shape has no finite bounding box");
            return OJCAD_STATUS_INVALID_SHAPE;
        }
        bounds.Get(out_bounds[0], out_bounds[1], out_bounds[2],
                   out_bounds[3], out_bounds[4], out_bounds[5]);
        for (int i = 0; i < 6; ++i) {
            if (!std::isfinite(out_bounds[i])) {
                set_error(out_error, OJCAD_STATUS_INVALID_SHAPE,
                          "shape bounding box contains a non-finite value");
                return OJCAD_STATUS_INVALID_SHAPE;
            }
        }
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" OJCAD_Status ojcad_shape_mass_properties(const OJCAD_Shape* shape,
                                                    double out_properties[5],
                                                    OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_properties == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and five-value mass properties output are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        GProp_GProps volume_properties;
        GProp_GProps surface_properties;
        const bool has_solid = TopExp_Explorer(shape->value, TopAbs_SOLID).More();
        if (has_solid) {
            BRepGProp::VolumeProperties(shape->value, volume_properties, 1e-9);
        }
        BRepGProp::SurfaceProperties(shape->value, surface_properties, 1e-9);
        const double volume = has_solid ? volume_properties.Mass() : 0.0;
        const double area = surface_properties.Mass();
        const gp_Pnt center = volume > 0.0 ? volume_properties.CentreOfMass()
                                           : surface_properties.CentreOfMass();
        out_properties[0] = volume;
        out_properties[1] = area;
        out_properties[2] = center.X();
        out_properties[3] = center.Y();
        out_properties[4] = center.Z();
        for (int i = 0; i < 5; ++i) {
            if (!std::isfinite(out_properties[i])) {
                set_error(out_error, OJCAD_STATUS_INVALID_SHAPE,
                          "shape mass properties contain a non-finite value");
                return OJCAD_STATUS_INVALID_SHAPE;
            }
        }
        if (out_properties[0] < 0.0 || out_properties[1] <= 0.0) {
            set_error(out_error, OJCAD_STATUS_INVALID_SHAPE,
                      "shape must have non-negative finite volume and positive finite surface area");
            return OJCAD_STATUS_INVALID_SHAPE;
        }
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" OJCAD_Status ojcad_shape_length_mm(const OJCAD_Shape* shape,
                                                double* out_length,
                                                OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_length == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output length pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        double length = 0.0;
        for (TopExp_Explorer explorer(shape->value, TopAbs_EDGE); explorer.More(); explorer.Next()) {
            const TopoDS_Edge edge = TopoDS::Edge(explorer.Current());
            double first = 0.0;
            double last = 0.0;
            const Handle(Geom_Curve) curve = BRep_Tool::Curve(edge, first, last);
            if (curve.IsNull()) continue;
            const GeomAdaptor_Curve adaptor(curve, first, last);
            length += GCPnts_AbscissaPoint::Length(adaptor, first, last, 1e-10);
        }
        if (!std::isfinite(length) || length <= 0.0) {
            set_error(out_error, OJCAD_STATUS_INVALID_SHAPE,
                      "shape has no finite positive edge length");
            return OJCAD_STATUS_INVALID_SHAPE;
        }
        *out_length = length;
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" OJCAD_Status ojcad_shape_transform_mm_degrees(
    const OJCAD_Shape* shape, double translate_x_mm, double translate_y_mm,
    double translate_z_mm, double rotate_x_deg, double rotate_y_deg,
    double rotate_z_deg, double uniform_scale, OJCAD_Shape** out_shape,
    OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    const double values[] = {translate_x_mm, translate_y_mm, translate_z_mm,
                              rotate_x_deg, rotate_y_deg, rotate_z_deg, uniform_scale};
    for (double value : values) {
        if (!std::isfinite(value)) {
            set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                      "transform values must be finite");
            return OJCAD_STATUS_INVALID_ARGUMENT;
        }
    }
    if (uniform_scale <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "uniform scale must be greater than zero");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }

    return create_shape([=]() {
        const gp_Pnt origin(0.0, 0.0, 0.0);
        const double radians_per_degree = std::acos(-1.0) / 180.0;
        TopoDS_Shape result = shape->value;

        gp_Trsf scaling;
        scaling.SetScale(origin, uniform_scale);
        BRepBuilderAPI_Transform scaled(result, scaling, true);
        if (!scaled.IsDone()) return TopoDS_Shape{};
        result = scaled.Shape();

        const gp_Dir axes[] = {gp_Dir(1.0, 0.0, 0.0), gp_Dir(0.0, 1.0, 0.0),
                               gp_Dir(0.0, 0.0, 1.0)};
        const double angles[] = {rotate_x_deg, rotate_y_deg, rotate_z_deg};
        for (int i = 0; i < 3; ++i) {
            if (angles[i] == 0.0) continue;
            gp_Trsf rotation;
            rotation.SetRotation(gp_Ax1(origin, axes[i]), angles[i] * radians_per_degree);
            BRepBuilderAPI_Transform rotated(result, rotation, true);
            if (!rotated.IsDone()) return TopoDS_Shape{};
            result = rotated.Shape();
        }

        gp_Trsf translation;
        translation.SetTranslation(gp_Vec(translate_x_mm, translate_y_mm, translate_z_mm));
        BRepBuilderAPI_Transform moved(result, translation, true);
        return moved.IsDone() ? moved.Shape() : TopoDS_Shape{};
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_shape_mirror(const OJCAD_Shape* shape,
                                              uint32_t plane_mask,
                                              OJCAD_Shape** out_shape,
                                              OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output shape pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (plane_mask > 7U) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "mirror plane mask may contain only X, Y, and Z plane bits");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }

    return create_shape([=]() {
        TopoDS_Shape result = shape->value;
        const gp_Pnt origin(0.0, 0.0, 0.0);
        const gp_Dir plane_normals[] = {gp_Dir(1.0, 0.0, 0.0),
                                        gp_Dir(0.0, 1.0, 0.0),
                                        gp_Dir(0.0, 0.0, 1.0)};
        for (uint32_t axis = 0; axis < 3; ++axis) {
            if ((plane_mask & (1U << axis)) == 0U) continue;
            gp_Trsf reflection;
            reflection.SetMirror(gp_Ax2(origin, plane_normals[axis]));
            BRepBuilderAPI_Transform mirrored(result, reflection, true);
            if (!mirrored.IsDone()) return TopoDS_Shape{};
            result = mirrored.Shape();
        }
        return result;
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_shape_boolean(const OJCAD_Shape* base,
                                              const OJCAD_Shape* tool,
                                              OJCAD_BooleanOperation operation,
                                              double tolerance_mm,
                                              OJCAD_Shape** out_shape,
                                              OJCAD_Error* out_error) {
    clear_error(out_error);
    if (base == nullptr || tool == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "base, tool, and output shape are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (!std::isfinite(tolerance_mm) || tolerance_mm <= 0.0) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "boolean tolerance must be finite and greater than zero in millimetres");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    if (operation != OJCAD_BOOLEAN_UNION && operation != OJCAD_BOOLEAN_DIFFERENCE &&
        operation != OJCAD_BOOLEAN_INTERSECTION) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "boolean operation must be union, difference, or intersection");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        const auto require_solid = [](const TopoDS_Shape& result) {
            if (result.IsNull()) return TopoDS_Shape{};
            for (TopExp_Explorer explorer(result, TopAbs_SOLID); explorer.More(); explorer.Next()) {
                return result;
            }
            return TopoDS_Shape{};
        };
        if (operation == OJCAD_BOOLEAN_UNION) {
            BRepAlgoAPI_Fuse builder(base->value, tool->value);
            builder.SetFuzzyValue(tolerance_mm);
            builder.SetRunParallel(Standard_False);
            builder.Build();
            return builder.IsDone() ? require_solid(builder.Shape()) : TopoDS_Shape{};
        }
        if (operation == OJCAD_BOOLEAN_DIFFERENCE) {
            BRepAlgoAPI_Cut builder(base->value, tool->value);
            builder.SetFuzzyValue(tolerance_mm);
            builder.SetRunParallel(Standard_False);
            builder.Build();
            return builder.IsDone() ? require_solid(builder.Shape()) : TopoDS_Shape{};
        }
        BRepAlgoAPI_Common builder(base->value, tool->value);
        builder.SetFuzzyValue(tolerance_mm);
        builder.SetRunParallel(Standard_False);
        builder.Build();
        return builder.IsDone() ? require_solid(builder.Shape()) : TopoDS_Shape{};
    }, out_shape, out_error);
}

namespace {
TopoDS_Shape make_linear_array(const TopoDS_Shape& source, uint32_t copies,
                               double step_x_mm, double step_y_mm, double step_z_mm) {
    TopoDS_Compound compound;
    BRep_Builder builder;
    builder.MakeCompound(compound);
    builder.Add(compound, source);
    for (uint32_t index = 1; index < copies; ++index) {
        const double factor = static_cast<double>(index);
        gp_Trsf translation;
        translation.SetTranslation(gp_Vec(step_x_mm * factor,
                                          step_y_mm * factor,
                                          step_z_mm * factor));
        BRepBuilderAPI_Transform duplicate(source, translation, true);
        if (!duplicate.IsDone()) return TopoDS_Shape{};
        builder.Add(compound, duplicate.Shape());
    }
    return compound;
}

TopoDS_Shape make_polar_array_z(const TopoDS_Shape& source, uint32_t copies,
                               double angle_step_deg) {
    TopoDS_Compound compound;
    BRep_Builder builder;
    builder.MakeCompound(compound);
    builder.Add(compound, source);
    const gp_Ax1 axis(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0));
    for (uint32_t index = 1; index < copies; ++index) {
        const double angle = static_cast<double>(index) * angle_step_deg * std::acos(-1.0) / 180.0;
        gp_Trsf rotation;
        rotation.SetRotation(axis, angle);
        BRepBuilderAPI_Transform duplicate(source, rotation, true);
        if (!duplicate.IsDone()) return TopoDS_Shape{};
        builder.Add(compound, duplicate.Shape());
    }
    return compound;
}
}

extern "C" OJCAD_Status ojcad_shape_linear_array(const OJCAD_Shape* shape, uint32_t copies,
                                                   double step_x_mm, double step_y_mm,
                                                   double step_z_mm,
                                                   OJCAD_Shape** out_shape,
                                                   OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (copies < 2 || copies > 64 || !std::isfinite(step_x_mm) ||
        !std::isfinite(step_y_mm) || !std::isfinite(step_z_mm) ||
        !std::isfinite(step_x_mm * (copies - 1)) ||
        !std::isfinite(step_y_mm * (copies - 1)) ||
        !std::isfinite(step_z_mm * (copies - 1)) ||
        (step_x_mm == 0.0 && step_y_mm == 0.0 && step_z_mm == 0.0)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "linear array requires 2..=64 copies and a finite non-zero millimetre step");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        return make_linear_array(shape->value, copies, step_x_mm, step_y_mm, step_z_mm);
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_shape_polar_array_z(const OJCAD_Shape* shape, uint32_t copies,
                                                    double angle_step_deg,
                                                    OJCAD_Shape** out_shape,
                                                    OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_shape == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_shape = nullptr;
    if (copies < 2 || copies > 64 || !std::isfinite(angle_step_deg) ||
        angle_step_deg == 0.0 || std::abs(angle_step_deg) >= 360.0 ||
        !std::isfinite(angle_step_deg * (copies - 1))) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "polar array requires 2..=64 copies and a non-zero angle below 360 degrees");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    return create_shape([=]() {
        return make_polar_array_z(shape->value, copies, angle_step_deg);
    }, out_shape, out_error);
}

extern "C" OJCAD_Status ojcad_shape_tessellate(const OJCAD_Shape* shape,
                                                 double linear_deflection_mm,
                                                 double angular_deflection_radians,
                                                 OJCAD_Mesh** out_mesh,
                                                 OJCAD_Error* out_error) {
    clear_error(out_error);
    if (shape == nullptr || out_mesh == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "shape and output mesh pointer are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_mesh = nullptr;
    if (!std::isfinite(linear_deflection_mm) || linear_deflection_mm <= 0.0 ||
        !std::isfinite(angular_deflection_radians) || angular_deflection_radians <= 0.0 ||
        angular_deflection_radians >= std::acos(-1.0)) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "mesh deflections must be finite and positive; angular deflection must be below pi");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    try {
        BRepMesh_IncrementalMesh mesher(shape->value, linear_deflection_mm,
                                        Standard_False, angular_deflection_radians,
                                        Standard_False);
        if (!mesher.IsDone()) {
            set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE,
                      "OCCT could not tessellate this shape");
            return OJCAD_STATUS_KERNEL_FAILURE;
        }

        std::unique_ptr<OJCAD_Mesh> mesh(new (std::nothrow) OJCAD_Mesh{});
        if (!mesh) {
            set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "could not allocate mesh handle");
            return OJCAD_STATUS_KERNEL_FAILURE;
        }
        for (TopExp_Explorer explorer(shape->value, TopAbs_FACE); explorer.More(); explorer.Next()) {
            const TopoDS_Face face = TopoDS::Face(explorer.Current());
            TopLoc_Location location;
            const Handle(Poly_Triangulation) triangulation =
                BRep_Tool::Triangulation(face, location);
            if (triangulation.IsNull()) continue;
            const uint64_t base = static_cast<uint64_t>(mesh->vertices_xyz.size() / 3);
            if (base + static_cast<uint64_t>(triangulation->NbNodes()) > UINT32_MAX) {
                set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE,
                          "tessellated shape exceeds the 32-bit mesh index limit");
                return OJCAD_STATUS_KERNEL_FAILURE;
            }
            mesh->vertices_xyz.reserve(mesh->vertices_xyz.size() +
                                       static_cast<size_t>(triangulation->NbNodes()) * 3);
            for (Standard_Integer node = 1; node <= triangulation->NbNodes(); ++node) {
                const gp_Pnt point = triangulation->Node(node).Transformed(location.Transformation());
                mesh->vertices_xyz.push_back(point.X());
                mesh->vertices_xyz.push_back(point.Y());
                mesh->vertices_xyz.push_back(point.Z());
            }
            mesh->triangle_indices.reserve(mesh->triangle_indices.size() +
                                            static_cast<size_t>(triangulation->NbTriangles()) * 3);
            for (Standard_Integer triangle = 1; triangle <= triangulation->NbTriangles(); ++triangle) {
                Standard_Integer a = 0;
                Standard_Integer b = 0;
                Standard_Integer c = 0;
                triangulation->Triangle(triangle).Get(a, b, c);
                if (face.Orientation() == TopAbs_REVERSED) std::swap(b, c);
                mesh->triangle_indices.push_back(static_cast<uint32_t>(base + a - 1));
                mesh->triangle_indices.push_back(static_cast<uint32_t>(base + b - 1));
                mesh->triangle_indices.push_back(static_cast<uint32_t>(base + c - 1));
            }
        }
        if (mesh->vertices_xyz.empty() || mesh->triangle_indices.empty()) {
            set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE,
                      "OCCT produced an empty mesh for this shape");
            return OJCAD_STATUS_KERNEL_FAILURE;
        }
        *out_mesh = mesh.release();
        return OJCAD_STATUS_OK;
    } catch (const std::exception& exception) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, exception.what());
        return OJCAD_STATUS_KERNEL_FAILURE;
    } catch (...) {
        set_error(out_error, OJCAD_STATUS_KERNEL_FAILURE, "unknown OCCT tessellation failure");
        return OJCAD_STATUS_KERNEL_FAILURE;
    }
}

extern "C" OJCAD_Status ojcad_mesh_counts(const OJCAD_Mesh* mesh,
                                            uint64_t* out_vertex_count,
                                            uint64_t* out_triangle_count,
                                            OJCAD_Error* out_error) {
    clear_error(out_error);
    if (mesh == nullptr || out_vertex_count == nullptr || out_triangle_count == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "mesh and vertex/triangle count outputs are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    *out_vertex_count = static_cast<uint64_t>(mesh->vertices_xyz.size() / 3);
    *out_triangle_count = static_cast<uint64_t>(mesh->triangle_indices.size() / 3);
    return OJCAD_STATUS_OK;
}

extern "C" OJCAD_Status ojcad_mesh_copy(const OJCAD_Mesh* mesh,
                                          double* out_vertices_xyz,
                                          uint64_t vertex_capacity,
                                          uint32_t* out_triangle_indices,
                                          uint64_t triangle_capacity,
                                          OJCAD_Error* out_error) {
    clear_error(out_error);
    if (mesh == nullptr || out_vertices_xyz == nullptr || out_triangle_indices == nullptr) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "mesh and vertex/triangle output buffers are required");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    const uint64_t vertex_count = static_cast<uint64_t>(mesh->vertices_xyz.size() / 3);
    const uint64_t triangle_count = static_cast<uint64_t>(mesh->triangle_indices.size() / 3);
    if (vertex_capacity < vertex_count || triangle_capacity < triangle_count) {
        set_error(out_error, OJCAD_STATUS_INVALID_ARGUMENT,
                  "mesh output buffers are smaller than the required counts");
        return OJCAD_STATUS_INVALID_ARGUMENT;
    }
    std::copy(mesh->vertices_xyz.begin(), mesh->vertices_xyz.end(), out_vertices_xyz);
    std::copy(mesh->triangle_indices.begin(), mesh->triangle_indices.end(), out_triangle_indices);
    return OJCAD_STATUS_OK;
}

extern "C" void ojcad_mesh_destroy(OJCAD_Mesh* mesh) { delete mesh; }

extern "C" void ojcad_shape_destroy(OJCAD_Shape* shape) { delete shape; }
