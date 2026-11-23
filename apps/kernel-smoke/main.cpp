#include "openjewelcad/cad_kernel.h"

#include <algorithm>
#include <cmath>
#include <iostream>
#include <iomanip>
#include <vector>

template <typename Create>
bool check_shape(const char* name, Create create, double expected_volume) {
    OJCAD_Shape* shape = nullptr;
    OJCAD_Error error{};
    if (create(&shape, &error) != OJCAD_STATUS_OK) {
        std::cerr << name << " creation failed: " << error.message << '\n';
        return false;
    }

    double volume = 0.0;
    int32_t valid = 0;
    const auto volume_status = ojcad_shape_volume_mm3(shape, &volume, &error);
    const auto valid_status = ojcad_shape_is_valid(shape, &valid, &error);
    ojcad_shape_destroy(shape);

    const double volume_tolerance = 1e-6 * std::max(1.0, std::abs(expected_volume));
    if (volume_status != OJCAD_STATUS_OK || valid_status != OJCAD_STATUS_OK ||
        valid != 1 || std::abs(volume - expected_volume) > volume_tolerance) {
        std::cerr << "OCCT " << name << " check failed: " << error.message
                  << "; volume=" << volume << ", valid=" << valid
                  << ", difference=" << std::abs(volume - expected_volume) << '\n';
        return false;
    }
    std::cout << "Valid OCCT " << name << "; volume=" << volume << " mm^3\n";
    return true;
}

double brilliant_volume(double diameter_mm, double height_mm, double table_diameter_mm) {
    constexpr int segments = 32;
    const double radius = diameter_mm * 0.5;
    const double table_radius = table_diameter_mm * 0.5;
    const double radii[] = {radius * 0.06, radius * 0.90, radius,
                            (radius + table_radius) * 0.5, table_radius};
    const double heights[] = {0.0, height_mm * 0.43, height_mm * 0.49,
                              height_mm * 0.77, height_mm};
    const auto area = [=](double section_radius) {
        return 0.5 * segments * section_radius * section_radius *
               std::sin(2.0 * std::acos(-1.0) / segments);
    };
    double volume = 0.0;
    for (int index = 0; index < 4; ++index) {
        const double bottom_area = area(radii[index]);
        const double top_area = area(radii[index + 1]);
        volume += (heights[index + 1] - heights[index]) / 3.0 *
                  (bottom_area + top_area + std::sqrt(bottom_area * top_area));
    }
    return volume;
}

int main() {
    const double rectangular_profile[] = {0.0, 0.0, 4.0, 0.0, 4.0, 3.0, 0.0, 3.0};
    const double revolve_profile[] = {3.0, -2.0, 5.0, -2.0, 5.0, 2.0, 3.0, 2.0};
    if (!check_shape("box", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_box_create(2.0, 3.0, 4.0, shape, error);
        }, 24.0) ||
        !check_shape("cylinder", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_cylinder_create(2.0, 5.0, shape, error);
        }, 20.0 * std::acos(-1.0)) ||
        !check_shape("cone", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_cone_create(2.0, 6.0, shape, error);
        }, 8.0 * std::acos(-1.0)) ||
        !check_shape("truncated cone", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_truncated_cone_create(3.0, 1.0, 4.0, shape, error);
        }, (52.0 / 3.0) * std::acos(-1.0)) ||
        !check_shape("ellipsoid", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_ellipsoid_create(4.0, 3.0, 2.0, shape, error);
        }, 32.0 * std::acos(-1.0)) ||
        !check_shape("pipe", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_pipe_create(3.0, 1.5, 4.0, shape, error);
        }, 27.0 * std::acos(-1.0)) ||
        !check_shape("sphere", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_sphere_create(3.0, shape, error);
        }, 36.0 * std::acos(-1.0)) ||
        !check_shape("torus", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_torus_create(5.0, 1.0, shape, error);
        }, 10.0 * std::pow(std::acos(-1.0), 2)) ||
        !check_shape("round brilliant", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_brilliant_gem_create(6.0, 3.6, 3.3, shape, error);
        }, brilliant_volume(6.0, 3.6, 3.3)) ||
        !check_shape("extruded polygon", [&rectangular_profile](OJCAD_Shape** shape,
                                                                 OJCAD_Error* error) {
            return ojcad_extruded_polygon_create(rectangular_profile, 4, 2.0, shape, error);
        }, 24.0) ||
        !check_shape("directional extruded polygon", [&rectangular_profile](OJCAD_Shape** shape,
                                                                               OJCAD_Error* error) {
            return ojcad_extruded_polygon_vector_create(rectangular_profile, 4, 5.0,
                                                         0.0, 3.0, 4.0, shape, error);
        }, 48.0) ||
        !check_shape("directional extruded circle", [](OJCAD_Shape** shape,
                                                          OJCAD_Error* error) {
            return ojcad_extruded_circle_vector_create(2.0, 5.0, 0.0, 3.0, 4.0,
                                                        shape, error);
        }, 16.0 * std::acos(-1.0)) ||
        !check_shape("directional extruded ellipse", [](OJCAD_Shape** shape,
                                                           OJCAD_Error* error) {
            return ojcad_extruded_ellipse_vector_create(4.0, 2.0, 5.0, 0.0, 3.0, 4.0,
                                                         shape, error);
        }, 32.0 * std::acos(-1.0)) ||
        !check_shape("revolved profile", [&revolve_profile](OJCAD_Shape** shape,
                                                              OJCAD_Error* error) {
            return ojcad_revolved_polygon_create(revolve_profile, 4, shape, error);
        }, 64.0 * std::acos(-1.0))) {
        return 1;
    }

    OJCAD_Shape* directional_polygon = nullptr;
    OJCAD_Error directional_error{};
    double directional_bounds[6]{};
    if (ojcad_extruded_polygon_vector_create(rectangular_profile, 4, 5.0, 0.0, 3.0, 4.0,
                                              &directional_polygon, &directional_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(directional_polygon, directional_bounds, &directional_error) !=
            OJCAD_STATUS_OK ||
        std::abs(directional_bounds[0]) > 1e-7 ||
        std::abs(directional_bounds[1]) > 1e-7 ||
        std::abs(directional_bounds[2]) > 1e-7 ||
        std::abs(directional_bounds[3] - 4.0) > 1e-7 ||
        std::abs(directional_bounds[4] - 6.0) > 1e-7 ||
        std::abs(directional_bounds[5] - 4.0) > 1e-7) {
        std::cerr << "OCCT directional polygon extrusion bounds failed: "
                  << directional_error.message << '\n';
        ojcad_shape_destroy(directional_polygon);
        return 1;
    }
    ojcad_shape_destroy(directional_polygon);

    OJCAD_Shape* flat_solid = nullptr;
    if (ojcad_extruded_circle_vector_create(2.0, 5.0, 1.0, 0.0, 0.0, &flat_solid,
                                             &directional_error) == OJCAD_STATUS_OK) {
        std::cerr << "OCCT accepted a zero-volume horizontal solid extrusion\n";
        ojcad_shape_destroy(flat_solid);
        return 1;
    }

    OJCAD_Error curve_error{};
    OJCAD_Shape* profile_curve = nullptr;
    OJCAD_Shape* profile_surface = nullptr;
    double curve_length = 0.0;
    double surface_properties[5]{};
    int32_t curve_valid = 0;
    int32_t surface_valid = 0;
    if (ojcad_closed_polyline_create(rectangular_profile, 4, &profile_curve, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(profile_curve, &curve_length, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(profile_curve, &curve_valid, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_planar_polygon_surface_create(rectangular_profile, 4, &profile_surface,
                                             &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(profile_surface, surface_properties, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(profile_surface, &surface_valid, &curve_error) != OJCAD_STATUS_OK ||
        curve_valid != 1 || surface_valid != 1 || std::abs(curve_length - 14.0) > 1e-8 ||
        std::abs(surface_properties[0]) > 1e-12 ||
        std::abs(surface_properties[1] - 12.0) > 1e-8 ||
        std::abs(surface_properties[2] - 2.0) > 1e-8 ||
        std::abs(surface_properties[3] - 1.5) > 1e-8) {
        std::cerr << "OCCT curve-to-surface profile workflow failed: "
                  << curve_error.message << '\n';
        ojcad_shape_destroy(profile_curve);
        ojcad_shape_destroy(profile_surface);
        return 1;
    }
    OJCAD_Mesh* surface_mesh = nullptr;
    uint64_t surface_vertex_count = 0;
    uint64_t surface_triangle_count = 0;
    if (ojcad_shape_tessellate(profile_surface, 0.05, 0.35, &surface_mesh, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_mesh_counts(surface_mesh, &surface_vertex_count, &surface_triangle_count,
                          &curve_error) != OJCAD_STATUS_OK ||
        surface_vertex_count < 4 || surface_triangle_count < 2) {
        std::cerr << "OCCT planar surface tessellation failed: " << curve_error.message << '\n';
        ojcad_mesh_destroy(surface_mesh);
        ojcad_shape_destroy(profile_curve);
        ojcad_shape_destroy(profile_surface);
        return 1;
    }
    std::cout << "Valid polyline curve; length=" << curve_length
              << " mm; planar face area=" << surface_properties[1] << " mm^2\n";
    ojcad_mesh_destroy(surface_mesh);
    ojcad_shape_destroy(profile_curve);
    ojcad_shape_destroy(profile_surface);

    const auto check_open_curve_surface = [](const char* name, const double* points,
                                             uint32_t point_count, double expected_area) {
        OJCAD_Shape* shape = nullptr;
        OJCAD_Error error{};
        double properties[5]{};
        int32_t valid = 0;
        if (ojcad_extruded_open_polyline_surface_create(points, point_count, 4.0, &shape, &error) !=
                OJCAD_STATUS_OK ||
            ojcad_shape_mass_properties(shape, properties, &error) != OJCAD_STATUS_OK ||
            ojcad_shape_is_valid(shape, &valid, &error) != OJCAD_STATUS_OK) {
            std::cerr << "Open-curve surface " << name << " failed: " << error.message << '\n';
            ojcad_shape_destroy(shape);
            return false;
        }
        ojcad_shape_destroy(shape);
        if (valid != 1 || std::abs(properties[1] - expected_area) > 1e-7) {
            std::cerr << "Open-curve surface " << name << " has invalid area or topology; valid="
                      << valid << ", volume=" << properties[0] << ", area=" << properties[1]
                      << ", expected=" << expected_area << '\n';
            return false;
        }
        std::cout << "Valid open-curve surface " << name << "; area=" << properties[1]
                  << " mm^2\n";
        return true;
    };
    const double line_profile[] = {-5.0, 0.0, 5.0, 0.0};
    const double open_path[] = {-5.0, 0.0, 0.0, 5.0, 5.0, 0.0};
    if (!check_open_curve_surface("line", line_profile, 2, 40.0) ||
        !check_open_curve_surface("polyline", open_path, 3, 40.0 * std::sqrt(2.0))) {
        return 1;
    }

    OJCAD_Shape* vector_surface = nullptr;
    double vector_surface_properties[5]{};
    double vector_surface_bounds[6]{};
    int32_t vector_surface_valid = 0;
    if (ojcad_extruded_open_polyline_surface_vector_create(
            line_profile, 2, 5.0, 0.0, 3.0, 4.0, &vector_surface, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(vector_surface, vector_surface_properties, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(vector_surface, vector_surface_bounds, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(vector_surface, &vector_surface_valid, &curve_error) !=
            OJCAD_STATUS_OK ||
        vector_surface_valid != 1 ||
        std::abs(vector_surface_properties[1] - 50.0) > 1e-7 ||
        std::abs(vector_surface_bounds[1]) > 1e-7 ||
        std::abs(vector_surface_bounds[4] - 3.0) > 1e-7 ||
        std::abs(vector_surface_bounds[5] - 4.0) > 1e-7) {
        std::cerr << "OCCT arbitrary-direction surface extrusion failed: "
                  << curve_error.message << '\n';
        ojcad_shape_destroy(vector_surface);
        return 1;
    }
    ojcad_shape_destroy(vector_surface);
    std::cout << "Valid arbitrary-direction surface extrusion; area="
              << vector_surface_properties[1] << " mm^2\n";

    const double revolved_open_profile[] = {5.0, -2.0, 5.0, 2.0};
    OJCAD_Shape* revolved_surface = nullptr;
    double revolved_surface_properties[5]{};
    int32_t revolved_surface_valid = 0;
    if (ojcad_revolved_open_polyline_surface_create(revolved_open_profile, 2,
                                                      &revolved_surface, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(revolved_surface, revolved_surface_properties,
                                     &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(revolved_surface, &revolved_surface_valid, &curve_error) !=
            OJCAD_STATUS_OK ||
        revolved_surface_valid != 1 ||
        std::abs(revolved_surface_properties[1] - 40.0 * std::acos(-1.0)) > 1e-7) {
        std::cerr << "OCCT revolved surface validation failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(revolved_surface);
        return 1;
    }
    std::cout << "Valid revolved open-profile surface; area="
              << revolved_surface_properties[1] << " mm^2\n";
    ojcad_shape_destroy(revolved_surface);

    OJCAD_Shape* circle_curve = nullptr;
    OJCAD_Shape* circle_surface = nullptr;
    OJCAD_Shape* circle_solid = nullptr;
    double circle_length = 0.0;
    double circle_properties[5]{};
    double circle_volume = 0.0;
    if (ojcad_circle_curve_create(2.0, &circle_curve, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(circle_curve, &circle_length, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_planar_circle_surface_create(2.0, &circle_surface, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(circle_surface, circle_properties, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_extruded_circle_create(2.0, 3.0, &circle_solid, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_volume_mm3(circle_solid, &circle_volume, &curve_error) != OJCAD_STATUS_OK ||
        std::abs(circle_length - 4.0 * std::acos(-1.0)) > 1e-7 ||
        std::abs(circle_properties[1] - 4.0 * std::acos(-1.0)) > 1e-7 ||
        std::abs(circle_volume - 12.0 * std::acos(-1.0)) > 1e-7) {
        std::cerr << "OCCT circle-to-cylinder workflow failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(circle_curve);
        ojcad_shape_destroy(circle_surface);
        ojcad_shape_destroy(circle_solid);
        return 1;
    }
    std::cout << "Valid circle stages; circumference=" << circle_length
              << " mm; disk area=" << circle_properties[1]
              << " mm^2; cylinder volume=" << circle_volume << " mm^3\n";
    ojcad_shape_destroy(circle_curve);
    ojcad_shape_destroy(circle_surface);
    ojcad_shape_destroy(circle_solid);

    OJCAD_Shape* arc_curve = nullptr;
    double arc_length = 0.0;
    double arc_bounds[6]{};
    if (ojcad_arc_curve_create(2.0, 270.0, 90.0, &arc_curve, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(arc_curve, &arc_length, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(arc_curve, arc_bounds, &curve_error) != OJCAD_STATUS_OK ||
        std::abs(arc_length - std::acos(-1.0)) > 1e-7 ||
        std::abs(arc_bounds[0]) > 1e-7 || std::abs(arc_bounds[1] + 2.0) > 1e-7 ||
        std::abs(arc_bounds[3] - 2.0) > 1e-7 || std::abs(arc_bounds[4]) > 1e-7) {
        std::cerr << "OCCT open arc validation failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(arc_curve);
        return 1;
    }
    std::cout << "Valid open arc; length=" << arc_length << " mm\n";
    ojcad_shape_destroy(arc_curve);

    OJCAD_Shape* arc_surface = nullptr;
    double arc_surface_properties[5]{};
    int32_t arc_surface_valid = 0;
    if (ojcad_extruded_arc_surface_create(2.0, 0.0, 90.0, 3.0, &arc_surface,
                                           &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(arc_surface, arc_surface_properties, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(arc_surface, &arc_surface_valid, &curve_error) != OJCAD_STATUS_OK ||
        arc_surface_valid != 1 ||
        std::abs(arc_surface_properties[1] - 3.0 * std::acos(-1.0)) > 1e-7) {
        std::cerr << "OCCT extruded arc surface validation failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(arc_surface);
        return 1;
    }
    std::cout << "Valid extruded arc surface; area=" << arc_surface_properties[1] << " mm^2\n";
    ojcad_shape_destroy(arc_surface);

    OJCAD_Shape* line_curve = nullptr;
    double line_length = 0.0;
    double line_bounds[6]{};
    if (ojcad_line_curve_create(-1.0, 2.0, 2.0, 6.0, &line_curve, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(line_curve, &line_length, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(line_curve, line_bounds, &curve_error) != OJCAD_STATUS_OK ||
        std::abs(line_length - 5.0) > 1e-9 || std::abs(line_bounds[0] + 1.0) > 1e-7 ||
        std::abs(line_bounds[1] - 2.0) > 1e-7 || std::abs(line_bounds[3] - 2.0) > 1e-7 ||
        std::abs(line_bounds[4] - 6.0) > 1e-7) {
        std::cerr << "OCCT open line validation failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(line_curve);
        return 1;
    }
    std::cout << "Valid open line; length=" << line_length << " mm\n";
    ojcad_shape_destroy(line_curve);

    const double open_polyline_points[] = {-1.0, 2.0, 2.0, 6.0, 6.0, 3.0};
    OJCAD_Shape* open_polyline = nullptr;
    double open_polyline_length = 0.0;
    double open_polyline_bounds[6]{};
    if (ojcad_open_polyline_curve_create(open_polyline_points, 3, &open_polyline, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(open_polyline, &open_polyline_length, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(open_polyline, open_polyline_bounds, &curve_error) !=
            OJCAD_STATUS_OK ||
        std::abs(open_polyline_length - 10.0) > 1e-9 ||
        std::abs(open_polyline_bounds[0] + 1.0) > 1e-7 ||
        std::abs(open_polyline_bounds[1] - 2.0) > 1e-7 ||
        std::abs(open_polyline_bounds[3] - 6.0) > 1e-7 ||
        std::abs(open_polyline_bounds[4] - 6.0) > 1e-7) {
        std::cerr << "OCCT open polyline validation failed: " << curve_error.message << '\n';
        ojcad_shape_destroy(open_polyline);
        return 1;
    }
    std::cout << "Valid open polyline; length=" << open_polyline_length << " mm\n";
    ojcad_shape_destroy(open_polyline);

    OJCAD_Shape* ellipse_curve = nullptr;
    OJCAD_Shape* ellipse_surface = nullptr;
    OJCAD_Shape* ellipse_solid = nullptr;
    double ellipse_length = 0.0;
    double ellipse_properties[5]{};
    double ellipse_volume = 0.0;
    if (ojcad_ellipse_curve_create(4.0, 2.0, &ellipse_curve, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_shape_length_mm(ellipse_curve, &ellipse_length, &curve_error) != OJCAD_STATUS_OK ||
        ojcad_planar_ellipse_surface_create(4.0, 2.0, &ellipse_surface, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_mass_properties(ellipse_surface, ellipse_properties, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_extruded_ellipse_create(4.0, 2.0, 3.0, &ellipse_solid, &curve_error) !=
            OJCAD_STATUS_OK ||
        ojcad_shape_volume_mm3(ellipse_solid, &ellipse_volume, &curve_error) != OJCAD_STATUS_OK ||
        ellipse_length <= 0.0 ||
        std::abs(ellipse_properties[1] - 8.0 * std::acos(-1.0)) > 1e-7 ||
        std::abs(ellipse_volume - 24.0 * std::acos(-1.0)) > 24.0 * std::acos(-1.0) * 1e-6) {
        std::cerr << std::setprecision(17) << "OCCT ellipse-to-prism workflow failed: " << curve_error.message
                  << "; perimeter=" << ellipse_length << " area=" << ellipse_properties[1]
                  << " volume=" << ellipse_volume << '\n';
        ojcad_shape_destroy(ellipse_curve);
        ojcad_shape_destroy(ellipse_surface);
        ojcad_shape_destroy(ellipse_solid);
        return 1;
    }
    std::cout << "Valid ellipse stages; perimeter=" << ellipse_length
              << " mm; face area=" << ellipse_properties[1]
              << " mm^2; prism volume=" << ellipse_volume << " mm^3\n";
    ojcad_shape_destroy(ellipse_curve);
    ojcad_shape_destroy(ellipse_surface);
    ojcad_shape_destroy(ellipse_solid);

    OJCAD_Shape* base = nullptr;
    OJCAD_Shape* transformed = nullptr;
    OJCAD_Error transform_error{};
    if (ojcad_box_create(2.0, 3.0, 4.0, &base, &transform_error) != OJCAD_STATUS_OK ||
        ojcad_shape_transform_mm_degrees(base, 1.0, -2.0, 3.0,
                                         0.0, 0.0, 90.0, 2.0,
                                         &transformed, &transform_error) != OJCAD_STATUS_OK) {
        std::cerr << "OCCT shape transform failed: " << transform_error.message << '\n';
        ojcad_shape_destroy(base);
        ojcad_shape_destroy(transformed);
        return 1;
    }
    ojcad_shape_destroy(base);
    double transformed_volume = 0.0;
    double bounds[6]{};
    double mass_properties[5]{};
    int32_t transformed_valid = 0;
    const auto transformed_volume_status =
        ojcad_shape_volume_mm3(transformed, &transformed_volume, &transform_error);
    const auto transformed_bounds_status =
        ojcad_shape_bounds_mm(transformed, bounds, &transform_error);
    const auto mass_properties_status =
        ojcad_shape_mass_properties(transformed, mass_properties, &transform_error);
    const auto transformed_valid_status =
        ojcad_shape_is_valid(transformed, &transformed_valid, &transform_error);
    OJCAD_Mesh* mesh = nullptr;
    uint64_t mesh_vertices = 0;
    uint64_t mesh_triangles = 0;
    const auto mesh_status = ojcad_shape_tessellate(transformed, 0.05, 0.35,
                                                     &mesh, &transform_error);
    const auto mesh_counts_status = mesh_status == OJCAD_STATUS_OK
        ? ojcad_mesh_counts(mesh, &mesh_vertices, &mesh_triangles, &transform_error)
        : mesh_status;
    std::vector<double> mesh_points(static_cast<size_t>(mesh_vertices) * 3);
    std::vector<uint32_t> mesh_indices(static_cast<size_t>(mesh_triangles) * 3);
    const auto mesh_copy_status = mesh_counts_status == OJCAD_STATUS_OK
        ? ojcad_mesh_copy(mesh, mesh_points.data(), mesh_vertices,
                          mesh_indices.data(), mesh_triangles, &transform_error)
        : mesh_counts_status;
    bool mesh_indices_valid = mesh_triangles > 0;
    for (const uint32_t index : mesh_indices) {
        mesh_indices_valid &= index < mesh_vertices;
    }
    ojcad_mesh_destroy(mesh);
    ojcad_shape_destroy(transformed);
    const double expected_bounds[6] = {-5.0, -2.0, 3.0, 1.0, 2.0, 11.0};
    const double expected_mass_properties[5] = {192.0, 208.0, -2.0, 0.0, 7.0};
    bool bounds_match = true;
    bool mass_properties_match = true;
    for (int i = 0; i < 6; ++i) {
        bounds_match &= std::abs(bounds[i] - expected_bounds[i]) < 1e-6;
    }
    for (int i = 0; i < 5; ++i) {
        mass_properties_match &=
            std::abs(mass_properties[i] - expected_mass_properties[i]) < 1e-6;
    }
    if (transformed_volume_status != OJCAD_STATUS_OK ||
        transformed_bounds_status != OJCAD_STATUS_OK ||
        mass_properties_status != OJCAD_STATUS_OK ||
        transformed_valid_status != OJCAD_STATUS_OK || transformed_valid != 1 ||
        std::abs(transformed_volume - 192.0) > 1e-6 || !bounds_match ||
        !mass_properties_match || mesh_status != OJCAD_STATUS_OK ||
        mesh_counts_status != OJCAD_STATUS_OK || mesh_copy_status != OJCAD_STATUS_OK ||
        !mesh_indices_valid) {
        std::cerr << "OCCT transform measurements failed: " << transform_error.message << '\n';
        return 1;
    }

    OJCAD_Shape* mirror_source = nullptr;
    OJCAD_Shape* mirrored = nullptr;
    OJCAD_Error mirror_error{};
    double mirrored_volume = 0.0;
    double mirrored_bounds[6]{};
    int32_t mirrored_valid = 0;
    if (ojcad_box_create(2.0, 3.0, 4.0, &mirror_source, &mirror_error) != OJCAD_STATUS_OK ||
        ojcad_shape_mirror(mirror_source, 1U, &mirrored, &mirror_error) != OJCAD_STATUS_OK ||
        ojcad_shape_volume_mm3(mirrored, &mirrored_volume, &mirror_error) != OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(mirrored, mirrored_bounds, &mirror_error) != OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(mirrored, &mirrored_valid, &mirror_error) != OJCAD_STATUS_OK) {
        std::cerr << "OCCT mirror failed: " << mirror_error.message << '\n';
        ojcad_shape_destroy(mirror_source);
        ojcad_shape_destroy(mirrored);
        return 1;
    }
    ojcad_shape_destroy(mirror_source);
    ojcad_shape_destroy(mirrored);
    const double expected_mirror_bounds[6] = {-2.0, 0.0, 0.0, 0.0, 3.0, 4.0};
    bool mirror_bounds_match = true;
    for (int i = 0; i < 6; ++i) {
        mirror_bounds_match &= std::abs(mirrored_bounds[i] - expected_mirror_bounds[i]) < 1e-7;
    }
    if (mirrored_valid != 1 || std::abs(mirrored_volume - 24.0) > 1e-7 ||
        !mirror_bounds_match) {
        std::cerr << "OCCT mirror measurements failed\n";
        return 1;
    }

    OJCAD_Shape* boolean_base = nullptr;
    OJCAD_Shape* boolean_tool_source = nullptr;
    OJCAD_Shape* boolean_tool = nullptr;
    OJCAD_Error boolean_error{};
    if (ojcad_box_create(2.0, 2.0, 2.0, &boolean_base, &boolean_error) != OJCAD_STATUS_OK ||
        ojcad_box_create(2.0, 2.0, 2.0, &boolean_tool_source, &boolean_error) != OJCAD_STATUS_OK ||
        ojcad_shape_transform_mm_degrees(boolean_tool_source, 1.0, 0.0, 0.0,
                                         0.0, 0.0, 0.0, 1.0,
                                         &boolean_tool, &boolean_error) != OJCAD_STATUS_OK) {
        std::cerr << "OCCT boolean inputs failed: " << boolean_error.message << '\n';
        ojcad_shape_destroy(boolean_base);
        ojcad_shape_destroy(boolean_tool_source);
        ojcad_shape_destroy(boolean_tool);
        return 1;
    }
    ojcad_shape_destroy(boolean_tool_source);
    const OJCAD_BooleanOperation boolean_operations[] = {
        OJCAD_BOOLEAN_UNION, OJCAD_BOOLEAN_DIFFERENCE, OJCAD_BOOLEAN_INTERSECTION};
    const double expected_boolean_volumes[] = {12.0, 4.0, 4.0};
    for (size_t i = 0; i < 3; ++i) {
        OJCAD_Shape* result = nullptr;
        double volume = 0.0;
        int32_t valid = 0;
        if (ojcad_shape_boolean(boolean_base, boolean_tool, boolean_operations[i], 1e-7,
                                &result, &boolean_error) != OJCAD_STATUS_OK ||
            ojcad_shape_volume_mm3(result, &volume, &boolean_error) != OJCAD_STATUS_OK ||
            ojcad_shape_is_valid(result, &valid, &boolean_error) != OJCAD_STATUS_OK ||
            valid != 1 || std::abs(volume - expected_boolean_volumes[i]) > 1e-7) {
            std::cerr << "OCCT boolean operation failed: " << boolean_error.message
                      << "; volume=" << volume << ", valid=" << valid << '\n';
            ojcad_shape_destroy(result);
            ojcad_shape_destroy(boolean_base);
            ojcad_shape_destroy(boolean_tool);
            return 1;
        }
        ojcad_shape_destroy(result);
    }
    ojcad_shape_destroy(boolean_base);
    ojcad_shape_destroy(boolean_tool);

    OJCAD_Shape* array_source = nullptr;
    OJCAD_Shape* linear_array = nullptr;
    OJCAD_Error array_error{};
    double array_volume = 0.0;
    double array_bounds[6]{};
    int32_t array_valid = 0;
    if (ojcad_box_create(2.0, 2.0, 2.0, &array_source, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_linear_array(array_source, 3, 4.0, 0.0, 0.0,
                                 &linear_array, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_volume_mm3(linear_array, &array_volume, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_bounds_mm(linear_array, array_bounds, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(linear_array, &array_valid, &array_error) != OJCAD_STATUS_OK ||
        array_valid != 1 || std::abs(array_volume - 24.0) > 1e-8 ||
        std::abs(array_bounds[3] - 10.0) > 1e-8) {
        std::cerr << "OCCT linear array failed: " << array_error.message << '\n';
        ojcad_shape_destroy(array_source);
        ojcad_shape_destroy(linear_array);
        return 1;
    }
    ojcad_shape_destroy(array_source);
    ojcad_shape_destroy(linear_array);

    OJCAD_Shape* polar_source = nullptr;
    OJCAD_Shape* polar_offset = nullptr;
    OJCAD_Shape* polar_array = nullptr;
    if (ojcad_box_create(1.0, 1.0, 1.0, &polar_source, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_transform_mm_degrees(polar_source, 2.0, 0.0, 0.0,
                                         0.0, 0.0, 0.0, 1.0,
                                         &polar_offset, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_polar_array_z(polar_offset, 4, 90.0,
                                  &polar_array, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_volume_mm3(polar_array, &array_volume, &array_error) != OJCAD_STATUS_OK ||
        ojcad_shape_is_valid(polar_array, &array_valid, &array_error) != OJCAD_STATUS_OK ||
        array_valid != 1 || std::abs(array_volume - 4.0) > 1e-8) {
        std::cerr << "OCCT polar array failed: " << array_error.message << '\n';
        ojcad_shape_destroy(polar_source);
        ojcad_shape_destroy(polar_offset);
        ojcad_shape_destroy(polar_array);
        return 1;
    }
    ojcad_shape_destroy(polar_source);
    ojcad_shape_destroy(polar_offset);
    ojcad_shape_destroy(polar_array);

    OJCAD_Shape* invalid_shape = nullptr;
    OJCAD_Error error{};
    if (ojcad_torus_create(1.0, 1.0, &invalid_shape, &error) !=
            OJCAD_STATUS_INVALID_ARGUMENT ||
        invalid_shape != nullptr) {
        std::cerr << "OCCT accepted an invalid torus profile\n";
        ojcad_shape_destroy(invalid_shape);
        return 1;
    }
    if (ojcad_truncated_cone_create(2.0, 2.0, 5.0, &invalid_shape, &error) !=
            OJCAD_STATUS_INVALID_ARGUMENT ||
        invalid_shape != nullptr) {
        std::cerr << "OCCT accepted a truncated cone with equal radii\n";
        ojcad_shape_destroy(invalid_shape);
        return 1;
    }
    if (ojcad_pipe_create(2.0, 2.0, 5.0, &invalid_shape, &error) !=
            OJCAD_STATUS_INVALID_ARGUMENT ||
        invalid_shape != nullptr) {
        std::cerr << "OCCT accepted a pipe with no wall thickness\n";
        ojcad_shape_destroy(invalid_shape);
        return 1;
    }
    const double self_intersecting_profile[] = {0.0, 0.0, 2.0, 2.0,
                                                 0.0, 2.0, 2.0, 0.0};
    if (ojcad_extruded_polygon_create(self_intersecting_profile, 4, 1.0,
                                      &invalid_shape, &error) != OJCAD_STATUS_INVALID_ARGUMENT ||
        invalid_shape != nullptr) {
        std::cerr << "OCCT accepted a self-intersecting sketch profile\n";
        ojcad_shape_destroy(invalid_shape);
        return 1;
    }
    return 0;
}
