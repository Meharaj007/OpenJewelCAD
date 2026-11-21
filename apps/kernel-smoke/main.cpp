#include "openjewelcad/cad_kernel.h"

#include <cmath>
#include <iostream>

int main() {
    OJCAD_Shape* box = nullptr;
    OJCAD_Error error{};
    if (ojcad_box_create(2.0, 3.0, 4.0, &box, &error) != OJCAD_STATUS_OK) {
        std::cerr << error.message << '\n';
        return 1;
    }

    double volume = 0.0;
    int32_t valid = 0;
    const auto volume_status = ojcad_shape_volume_mm3(box, &volume, &error);
    const auto valid_status = ojcad_shape_is_valid(box, &valid, &error);
    ojcad_shape_destroy(box);

    if (volume_status != OJCAD_STATUS_OK || valid_status != OJCAD_STATUS_OK ||
        valid != 1 || std::abs(volume - 24.0) > 1e-9) {
        std::cerr << "OCCT box smoke check failed: " << error.message << '\n';
        return 1;
    }
    std::cout << "Valid OCCT box; volume=" << volume << " mm^3\n";
    return 0;
}
