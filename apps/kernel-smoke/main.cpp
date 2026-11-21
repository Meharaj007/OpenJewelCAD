#include "openjewelcad/cad_kernel.h"

#include <cmath>
#include <iostream>

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

    if (volume_status != OJCAD_STATUS_OK || valid_status != OJCAD_STATUS_OK ||
        valid != 1 || std::abs(volume - expected_volume) > 1e-7) {
        std::cerr << "OCCT " << name << " check failed: " << error.message
                  << "; volume=" << volume << '\n';
        return false;
    }
    std::cout << "Valid OCCT " << name << "; volume=" << volume << " mm^3\n";
    return true;
}

int main() {
    if (!check_shape("box", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_box_create(2.0, 3.0, 4.0, shape, error);
        }, 24.0) ||
        !check_shape("cylinder", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_cylinder_create(2.0, 5.0, shape, error);
        }, 20.0 * std::acos(-1.0)) ||
        !check_shape("sphere", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_sphere_create(3.0, shape, error);
        }, 36.0 * std::acos(-1.0)) ||
        !check_shape("torus", [](OJCAD_Shape** shape, OJCAD_Error* error) {
            return ojcad_torus_create(5.0, 1.0, shape, error);
        }, 10.0 * std::pow(std::acos(-1.0), 2))) {
        return 1;
    }

    OJCAD_Shape* invalid_shape = nullptr;
    OJCAD_Error error{};
    if (ojcad_torus_create(1.0, 1.0, &invalid_shape, &error) !=
            OJCAD_STATUS_INVALID_ARGUMENT ||
        invalid_shape != nullptr) {
        std::cerr << "OCCT accepted an invalid torus profile\n";
        ojcad_shape_destroy(invalid_shape);
        return 1;
    }
    return 0;
}
