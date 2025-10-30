#ifndef TESTLIB_H
#define TESTLIB_H

#include <stdint.h>
#include <stddef.h>

// enums

typedef enum {
    STATUS_OK = 0,
    STATUS_ERROR = 1,
    STATUS_PENDING = 2,
    STATUS_TIMEOUT = 3
} Status;

typedef enum {
    COLOR_RED,
    COLOR_GREEN,
    COLOR_BLUE
} Color;

// simple structs

typedef struct {
    int x;
    int y;
} Point;

typedef struct {
    float width;
    float height;
} Rectangle;

// nested structs

typedef struct {
    Point top_left;
    Point bottom_right;
} BoundingBox;

// struct with various types

typedef struct {
    char name[64];
    int age;
    float salary;
    double balance;
    Status status;
    uint8_t flags;
    int64_t timestamp;
} Person;

// opaque types (forward declarations)

typedef struct InternalState InternalState;

// unions

typedef union {
    int as_int;
    float as_float;
    char as_bytes[4];
} DataUnion;

// function pointer types

typedef void (*Callback)(int code, void* userdata);
typedef int (*Comparator)(const void* a, const void* b);

// exported api functions

// basic types - primitives
__attribute__((visibility("default")))
void simple_void_function(void);

__attribute__((visibility("default")))
int return_int(void);

__attribute__((visibility("default")))
int add_two_ints(int a, int b);

__attribute__((visibility("default")))
float multiply_floats(float a, float b);

__attribute__((visibility("default")))
double compute_double(double x, double y, double z);

// different integer types
__attribute__((visibility("default")))
uint8_t process_byte(uint8_t value);

__attribute__((visibility("default")))
int64_t process_long(int64_t value);

__attribute__((visibility("default")))
size_t get_size(void);

// pointers
__attribute__((visibility("default")))
void modify_value(int* ptr);

__attribute__((visibility("default")))
const char* get_string(void);

__attribute__((visibility("default")))
void process_buffer(char* buffer, size_t length);

__attribute__((visibility("default")))
int* allocate_array(size_t count);

// double pointers
__attribute__((visibility("default")))
void allocate_matrix(int** matrix, int rows, int cols);

// const pointers
__attribute__((visibility("default")))
int sum_array(const int* arr, size_t length);

__attribute__((visibility("default")))
void print_string(const char* str);

// enums
__attribute__((visibility("default")))
Status get_status(void);

__attribute__((visibility("default")))
void set_status(Status s);

__attribute__((visibility("default")))
Color blend_colors(Color c1, Color c2);

// simple structs
__attribute__((visibility("default")))
Point create_point(int x, int y);

__attribute__((visibility("default")))
void move_point(Point* p, int dx, int dy);

__attribute__((visibility("default")))
float calculate_distance(Point p1, Point p2);

__attribute__((visibility("default")))
Rectangle create_rectangle(float w, float h);

// passing structs by value
__attribute__((visibility("default")))
Point add_points(Point p1, Point p2);

// nested structs
__attribute__((visibility("default")))
BoundingBox create_bounding_box(Point tl, Point br);

__attribute__((visibility("default")))
int is_point_inside(BoundingBox box, Point p);

// complex structs
__attribute__((visibility("default")))
Person* create_person(const char* name, int age);

__attribute__((visibility("default")))
void destroy_person(Person* p);

__attribute__((visibility("default")))
void update_person_status(Person* p, Status new_status);

// opaque types
__attribute__((visibility("default")))
InternalState* init_state(void);

__attribute__((visibility("default")))
void cleanup_state(InternalState* state);

__attribute__((visibility("default")))
int process_state(InternalState* state, int value);

// unions
__attribute__((visibility("default")))
DataUnion create_data_union(int value);

__attribute__((visibility("default")))
float get_float_from_union(DataUnion data);

// function pointers
__attribute__((visibility("default")))
void register_callback(Callback cb, void* userdata);

__attribute__((visibility("default")))
void sort_array(int* arr, size_t count, Comparator cmp);

// mixed complex parameters
__attribute__((visibility("default")))
Status process_person_batch(Person** people, size_t count, Callback on_complete);

__attribute__((visibility("default")))
void complex_function(
    const char* name,
    Point* points,
    size_t point_count,
    Rectangle bounds,
    Status* out_status
);

// variadic functions (if you want to test these)
__attribute__((visibility("default")))
int sum_varargs(int count, ...);

// arrays as parameters (decays to pointer)
__attribute__((visibility("default")))
void process_fixed_array(int arr[10]);

__attribute__((visibility("default")))
void process_2d_array(int arr[5][5]);

// internal/hidden functions

// These are helper functions without visibility attribute
// They should NOT appear in your exported symbol list
void internal_helper(void);
int internal_compute(int a, int b);
void internal_process_data(const char* data, size_t len);

#endif // TESTLIB_H