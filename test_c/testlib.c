#include "testlib.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdarg.h>

// opaque type definition

struct InternalState
{
    int counter;
    double value;
    char buffer[256];
};

// internal/hidden functions

void internal_helper(void)
{
    printf("This is an internal helper function\n");
}

int internal_compute(int a, int b)
{
    return (a * b) + (a - b);
}

void internal_process_data(const char *data, size_t len)
{
    // just a dummy internal function
    (void)data;
    for (size_t i = 0; i < len; i++)
    {
        // do nothing
    }
}

// exported api implementations

void simple_void_function(void)
{
    printf("Simple void function called\n");
}

int return_int(void)
{
    return 42;
}

int add_two_ints(int a, int b)
{
    return a + b;
}

float multiply_floats(float a, float b)
{
    return a * b;
}

double compute_double(double x, double y, double z)
{
    return (x + y) * z;
}

uint8_t process_byte(uint8_t value)
{
    return value ^ 0xFF;
}

int64_t process_long(int64_t value)
{
    return value * 2;
}

size_t get_size(void)
{
    return sizeof(Person);
}

void modify_value(int *ptr)
{
    if (ptr)
    {
        *ptr = *ptr + 10;
    }
}

const char *get_string(void)
{
    return "Hello from testlib";
}

void process_buffer(char *buffer, size_t length)
{
    for (size_t i = 0; i < length; i++)
    {
        buffer[i] = (char)(buffer[i] + 1);
    }
}

int *allocate_array(size_t count)
{
    return (int *)malloc(count * sizeof(int));
}

void allocate_matrix(int **matrix, int rows, int cols)
{
    if (matrix)
    {
        for (int i = 0; i < rows; i++)
        {
            matrix[i] = (int *)malloc(cols * sizeof(int));
        }
    }
}

int sum_array(const int *arr, size_t length)
{
    int sum = 0;
    for (size_t i = 0; i < length; i++)
    {
        sum += arr[i];
    }
    return sum;
}

void print_string(const char *str)
{
    if (str)
    {
        printf("%s\n", str);
    }
}

Status get_status(void)
{
    return STATUS_OK;
}

void set_status(Status s)
{
    // store status somewhere
    (void)s;
}

Color blend_colors(Color c1, Color c2)
{
    // simple logic
    if (c1 == c2)
        return c1;
    return COLOR_GREEN;
}

Point create_point(int x, int y)
{
    Point p = {x, y};
    return p;
}

void move_point(Point *p, int dx, int dy)
{
    if (p)
    {
        p->x += dx;
        p->y += dy;
    }
}

float calculate_distance(Point p1, Point p2)
{
    int dx = p2.x - p1.x;
    int dy = p2.y - p1.y;
    return sqrtf((float)(dx * dx + dy * dy));
}

Rectangle create_rectangle(float w, float h)
{
    Rectangle r = {w, h};
    return r;
}

Point add_points(Point p1, Point p2)
{
    Point result = {p1.x + p2.x, p1.y + p2.y};
    return result;
}

BoundingBox create_bounding_box(Point tl, Point br)
{
    BoundingBox box = {tl, br};
    return box;
}

int is_point_inside(BoundingBox box, Point p)
{
    return (p.x >= box.top_left.x && p.x <= box.bottom_right.x &&
            p.y >= box.top_left.y && p.y <= box.bottom_right.y);
}

Person *create_person(const char *name, int age)
{
    Person *p = (Person *)malloc(sizeof(Person));
    if (p)
    {
        strncpy(p->name, name, sizeof(p->name) - 1);
        p->name[sizeof(p->name) - 1] = '\0';
        p->age = age;
        p->salary = 0.0f;
        p->balance = 0.0;
        p->status = STATUS_OK;
        p->flags = 0;
        p->timestamp = 0;
    }
    return p;
}

void destroy_person(Person *p)
{
    free(p);
}

void update_person_status(Person *p, Status new_status)
{
    if (p)
    {
        p->status = new_status;
    }
}

InternalState *init_state(void)
{
    InternalState *state = (InternalState *)malloc(sizeof(InternalState));
    if (state)
    {
        state->counter = 0;
        state->value = 0.0;
        memset(state->buffer, 0, sizeof(state->buffer));
    }
    return state;
}

void cleanup_state(InternalState *state)
{
    free(state);
}

int process_state(InternalState *state, int value)
{
    if (state)
    {
        state->counter += value;
        return state->counter;
    }
    return -1;
}

DataUnion create_data_union(int value)
{
    DataUnion data;
    data.as_int = value;
    return data;
}

float get_float_from_union(DataUnion data)
{
    return data.as_float;
}

void register_callback(Callback cb, void *userdata)
{
    if (cb)
    {
        cb(0, userdata);
    }
}

void sort_array(int *arr, size_t count, Comparator cmp)
{
    // simple bubble sort for demonstration
    if (!arr || !cmp)
        return;

    for (size_t i = 0; i < count - 1; i++)
    {
        for (size_t j = 0; j < count - i - 1; j++)
        {
            if (cmp(&arr[j], &arr[j + 1]) > 0)
            {
                int temp = arr[j];
                arr[j] = arr[j + 1];
                arr[j + 1] = temp;
            }
        }
    }
}

Status process_person_batch(Person **people, size_t count, Callback on_complete)
{
    if (!people)
        return STATUS_ERROR;

    for (size_t i = 0; i < count; i++)
    {
        if (people[i])
        {
            people[i]->status = STATUS_OK;
        }
    }

    if (on_complete)
    {
        on_complete(0, NULL);
    }

    return STATUS_OK;
}

void complex_function(
    const char *name,
    Point *points,
    size_t point_count,
    Rectangle bounds,
    Status *out_status)
{
    internal_helper();
    (void)bounds;

    if (!name || !points || !out_status)
    {
        if (out_status)
            *out_status = STATUS_ERROR;
        return;
    }

    // do some processing
    for (size_t i = 0; i < point_count; i++)
    {
        if (points[i].x < 0 || points[i].y < 0)
        {
            *out_status = STATUS_ERROR;
            return;
        }
    }

    *out_status = STATUS_OK;
}

int sum_varargs(int count, ...)
{
    va_list args;
    va_start(args, count);

    int sum = 0;
    for (int i = 0; i < count; i++)
    {
        sum += va_arg(args, int);
    }

    va_end(args);
    return sum;
}

void process_fixed_array(int arr[10])
{
    for (int i = 0; i < 10; i++)
    {
        arr[i] *= 2;
    }
}

void process_2d_array(int arr[5][5])
{
    for (int i = 0; i < 5; i++)
    {
        for (int j = 0; j < 5; j++)
        {
            arr[i][j] = i * 5 + j;
        }
    }
}