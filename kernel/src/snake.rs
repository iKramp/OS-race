use std::Vec;

pub struct State {
    width: i32,
    height: i32,
    snake: Vec<(i32, i32)>,
    food: (i32, i32),
    direction: (i32, i32),
}

pub fn init() -> State {
    let state = State {
        width: 80,
        height: 25,
        snake: Vec::new(), // ![(40, 12), (41, 12), (42, 12)],
        food: (10, 10),
        direction: (1, 0),
    };

    crate::vga::clear_screen();

    state
}

pub fn tick(state: &mut State) {
    move_snake(state);
    // check if snake is eating food
    // check if snake is colliding with itself
    // check if snake is colliding with wall
    // generate new food
}

fn move_snake(state: &mut State) {
    state.snake.insert(0, (state.snake[0].0 + state.direction.0, state.snake[0].1 + state.direction.1));
    if !((state.snake.first().unwrap().0 == state.food.0) && (state.snake.first().unwrap().1 == state.food.1)) {
        state.snake.pop();
        //clear old pos from screen
    }
}
