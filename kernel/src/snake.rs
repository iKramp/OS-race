use crate::vga::vga_driver;
use std::Vec;

const PIXELS_PER_SQUARE: i32 = 10;

pub struct State {
    width: i32,
    height: i32,
    snake: Vec<(i32, i32)>,
    food: (i32, i32),
    direction: (i32, i32),
}

pub fn init() -> State {
    unsafe {
        let mut state = State {
            width: crate::vga::vga_driver::VGA_BINDING.width as i32 / PIXELS_PER_SQUARE,
            height: crate::vga::vga_driver::VGA_BINDING.height as i32 / PIXELS_PER_SQUARE,
            snake: Vec::new(),
            food: (10, 10),
            direction: (1, 0),
        };
        state.snake.push((40, 12));
        state.snake.push((41, 12));
        state.snake.push((42, 12));

        crate::vga::clear_screen();

        draw_food(&state);

        state
    }
}

pub fn tick(state: &mut State) {
    rotate(state);
    move_snake(state);
    if !((state.snake.first().unwrap().0 == state.food.0) && (state.snake.first().unwrap().1 == state.food.1)) {
        delete_last_snake_part(state);
    } else {
        move_food(state);
    }
    // check if snake is colliding with itself
    // check if snake is colliding with wall
}

fn draw_food(state: &State) {
    vga_driver::draw_rectangle(
        (state.food.0 * PIXELS_PER_SQUARE) as usize,
        (state.food.1 * PIXELS_PER_SQUARE) as usize,
        PIXELS_PER_SQUARE as usize,
        PIXELS_PER_SQUARE as usize,
        (0, 0, 255),
    );
}

fn move_snake(state: &mut State) {
    state.snake.insert(
        0,
        (state.snake[0].0 + state.direction.0, state.snake[0].1 + state.direction.1),
    );
    if state.snake[0].0 < 0 {
        state.snake[0].0 = state.width - 1;
    }
    if state.snake[0].0 >= state.width {
        state.snake[0].0 = 0;
    }
    if state.snake[0].1 < 0 {
        state.snake[0].1 = state.height - 1;
    }
    if state.snake[0].1 >= state.height {
        state.snake[0].1 = 0;
    }
    vga_driver::draw_rectangle(
        (state.snake[0].0 * PIXELS_PER_SQUARE) as usize,
        (state.snake[0].1 * PIXELS_PER_SQUARE) as usize,
        PIXELS_PER_SQUARE as usize,
        PIXELS_PER_SQUARE as usize,
        (255, 255, 255),
    );
}

fn delete_last_snake_part(state: &mut State) {
    state.snake.pop();
    vga_driver::draw_rectangle(
        (state.snake.last().unwrap().0 * PIXELS_PER_SQUARE) as usize,
        (state.snake.last().unwrap().1 * PIXELS_PER_SQUARE) as usize,
        PIXELS_PER_SQUARE as usize,
        PIXELS_PER_SQUARE as usize,
        (0, 0, 0),
    );
}

fn move_food(state: &mut State) {
    while state.snake.iter().any(|&x| x == state.food) {
        state.food.0 = (state.food.0 + 2131324) % state.width;
        state.food.1 = (state.food.1 + 2131324) % state.height;
    }
    draw_food(state);
}

fn rotate(state: &mut State) {
    unsafe {
        if crate::keyboard::KEY_STATES[72] {
            // up
            state.direction = (0, -1);
        }
        if crate::keyboard::KEY_STATES[80] {
            // down
            state.direction = (0, 1);
        }
        if crate::keyboard::KEY_STATES[75] {
            // left
            state.direction = (-1, 0);
        }
        if crate::keyboard::KEY_STATES[77] {
            // right
            state.direction = (1, 0);
        }
    }
}
