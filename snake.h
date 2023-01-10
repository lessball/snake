#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


typedef struct SnakeHead SnakeHead;

typedef struct Vector2 {
  float x;
  float y;
} Vector2;

typedef struct SnakeBody {
  float delay;
  float distance;
  struct Vector2 position;
  struct Vector2 target;
} SnakeBody;

void snake_drop(struct SnakeHead *head);

void snake_move_head(struct SnakeHead *head, struct Vector2 position, double dt);

struct SnakeHead *snake_new(float max_delay, float max_distance);

void snake_reset(struct SnakeHead *head, struct Vector2 position);

void snake_solve_body(struct SnakeHead *head,
                      struct SnakeBody *bodies,
                      size_t num_bodies,
                      float max_move,
                      float min_move,
                      float radius);
