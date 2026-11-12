cmake_minimum_required(VERSION 3.16)
project(soplos-look-manager-style-preview LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC OFF)

# The helper runs with the libraries of the system, not with the ones that were
# used to build it (the headers of KDecoration3 may come from build/deps).
set(CMAKE_BUILD_WITH_INSTALL_RPATH ON)
set(CMAKE_INSTALL_RPATH "")

find_package(Qt6 REQUIRED COMPONENTS Widgets Quick)
find_package(KF6CoreAddons REQUIRED)
find_package(KF6Config REQUIRED)
find_package(KF6Package REQUIRED)
find_package(KF6ColorScheme REQUIRED)
find_package(KF6Svg REQUIRED)
find_package(KDecoration3 REQUIRED)

add_executable(soplos-look-manager-style-preview main.cpp)
target_link_libraries(soplos-look-manager-style-preview PRIVATE
    Qt6::Widgets Qt6::Quick KF6::CoreAddons KF6::ConfigCore KF6::Package KF6::ColorScheme KF6::Svg
    KDecoration3::KDecoration KDecoration3::KDecoration3Private)
