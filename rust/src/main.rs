extern crate minifb;
use minifb::{Key, Window, WindowOptions};
use rand::prelude::*;
// use std::num::Wrapping;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

pub struct KeyPad {
    keys: [bool; 16],
}

impl Default for KeyPad {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyPad {
    pub fn new() -> KeyPad {
        KeyPad {
            keys: [
                false, false, false, false,
                false, false, false, false,
                false, false, false, false,
                false, false, false, false,
            ]
        }
    }

    pub fn update_keys(&mut self, keys: Vec<Key>) {
        self.keys.iter_mut().for_each(|m| *m = false);
        for key in keys {
            match key {
                Key::Key1 => { self.keys[0x1] = true }
                Key::Key2 => { self.keys[0x2] = true }
                Key::Key3 => { self.keys[0x3] = true }
                Key::Key4 => { self.keys[0xC] = true }

                Key::Q => { self.keys[0x4] = true }
                Key::W => { self.keys[0x5] = true }
                Key::E => { self.keys[0x6] = true }
                Key::R => { self.keys[0xD] = true }

                Key::A => { self.keys[0x7] = true }
                Key::S => { self.keys[0x8] = true }
                Key::D => { self.keys[0x9] = true }
                Key::F => { self.keys[0xE] = true }

                Key::Z => { self.keys[0xA] = true }
                Key::X => { self.keys[0x0] = true }
                Key::C => { self.keys[0xB] = true }
                Key::V => { self.keys[0xF] = true }
                _ => { } // ignore all other keys
            }
        }
    }

    pub fn is_key_pressed(&self, key_code: u8) -> bool {
        self.keys[key_code as usize]
    }
}

pub struct Display {
    screen: [u8; WIDTH * HEIGHT],
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

impl Display {
    pub fn new() -> Display {
        Display {
            screen: [0; WIDTH * HEIGHT],
        }
    }

    pub fn get_index_from_coords(x: usize, y: usize) -> usize {
        y * WIDTH + x
    }

    pub fn draw(&mut self, byte: u8, x: u8, y: u8) -> bool {
        let mut erased = false;
        let mut coord_x = x as usize;
        let mut coord_y = y as usize;
        let mut b = byte;

        for _ in 0..8 {
            coord_x %= WIDTH;
            coord_y %= HEIGHT;
            let index = Display::get_index_from_coords(coord_x, coord_y);
            let bit = (b & 0b1000_0000) >> 7;
            let prev_value = self.screen[index];
            self.screen[index] ^= bit;

            if prev_value == 1 && self.screen[index] == 0 {
                erased = true;
            }

            coord_x += 1;
            b <<= 1;
        }

        erased
    }

    pub fn clear(&mut self) {
        for pixel in self.screen.iter_mut() {
            *pixel = 0;
        }
    }

    pub fn get_display_buffer(&self) -> &[u8] {
        &self.screen
    }
}

struct Chip8 {
    vx: [u8; 16],  // 16 8-bit registers
    pc: u16, // program counter
    i: u16,
    sound_timer: u8,
    delay_timer: u8,
    stack: Vec<u16>,
    screen: Display,
    keypad: KeyPad,
    memory: [u8; 4096],
    draw: bool
}

// screen 63,31

impl Chip8 {
    fn new() -> Chip8 {
        let mut memory = [0; 4096];

        let sprites: [[u8; 5]; 16] = [
            [0xF0, 0x90, 0x90, 0x90, 0xF0],
            [0x20, 0x60, 0x20, 0x20, 0x70],
            [0xF0, 0x10, 0xF0, 0x80, 0xF0],
            [0xF0, 0x10, 0xF0, 0x10, 0xF0],
            [0x90, 0x90, 0xF0, 0x10, 0x10],
            [0xF0, 0x80, 0xF0, 0x10, 0xF0],
            [0xF0, 0x80, 0xF0, 0x90, 0xF0],
            [0xF0, 0x10, 0x20, 0x40, 0x40],
            [0xF0, 0x90, 0xF0, 0x90, 0xF0],
            [0xF0, 0x90, 0xF0, 0x10, 0xF0],
            [0xF0, 0x90, 0xF0, 0x90, 0x90],
            [0xE0, 0x90, 0xE0, 0x90, 0xE0],
            [0xF0, 0x80, 0x80, 0x80, 0xF0],
            [0xE0, 0x90, 0x90, 0x90, 0xE0],
            [0xF0, 0x80, 0xF0, 0x80, 0xF0],
            [0xF0, 0x80, 0xF0, 0x80, 0x80],
        ];

        let mut i = 0;
        for sprite in &sprites {
            for ch in sprite {
                memory[i] = *ch;
                i += 1;
            }
        }

        Chip8 {
            vx: [0; 16],
            pc: 0x200,
            i: 0,
            stack: vec![],
            screen: Display::new(),
            delay_timer: 0,
            sound_timer: 0,
            memory,
            keypad: KeyPad::new(),
            draw: false
        }
    }

    fn load_program(&mut self, program: Vec<u8>) {
        let mut data = vec![0; 0x200];
        for byte in program {
            data.push(byte);
        }
        for (index, &byte) in data.iter().enumerate() {
            if index >= 0x200 {
                self.memory[index] = byte;
            }
        }    }

    fn step(&mut self) {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[(self.pc + 1) as usize] as u16;
        let instruction: u16 = (hi << 8) | lo;

        let nnn = instruction & 0x0FFF;
        let nn = (instruction & 0x0FF) as u8;
        let n = (instruction & 0x00F) as u8;
        let x = ((instruction & 0x0F00) >> 8) as u8;
        let y = ((instruction & 0x00F0) >> 4) as u8;
        match (instruction & 0xF000) >> 12 {
            0x0 => {
                match nn {
                    0xE0 => {
                        self.screen.clear();
                        self.pc += 2;
                    }
                    0xEE => {
                        //return from subroutine
                        let addr = self.stack.pop().unwrap();
                        self.pc = addr;
                    }
                    _ => unreachable!(
                        "Unrecognized 0x00** instruction {:#X}:{:#X}",
                        self.pc,
                        instruction
                    ),
                }
            }
            0x1 => {
                self.pc = nnn;
            },
            0x2 => {
                self.stack.push(self.pc + 2);
                self.pc = nnn;
            },
            0x3 => {
                if self.vx[x as usize] == nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x4 => {
                if self.vx[x as usize] != nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x5 => {
                if self.vx[x as usize] == self.vx[y as usize] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x6 => {
                self.vx[x as usize] = nn;
                self.pc += 2;
            },
            0x7 => {
                self.vx[x as usize] = self.vx[x as usize].wrapping_add(nn);
                self.pc += 2;
            },
            0x8 => {
                match n {
                    0 => {
                        self.vx[x as usize] = self.vx[y as usize];
                        self.pc += 2;
                    }
                    1 => {
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        self.vx[x as usize] = vx | vy;
                        self.pc += 2;
                    }
                    2 => {
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        self.vx[x as usize] = vx & vy;
                        self.pc += 2;
                    }
                    3 => {
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        self.vx[x as usize] = vx ^ vy;
                        self.pc += 2;
                    }
                    4 => {
                        // TODO: setting flags
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        let result = vx.overflowing_add(vy);
                        self.vx[x as usize] = result.0;
                        if result.1 {
                            self.vx[0xF_usize] = 1;
                        } else {
                            self.vx[0xF_usize] = 0;
                        }
                        self.pc += 2;
                    }
                    5 => {
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        let result = vx.overflowing_sub(vy);
                        self.vx[x as usize] = result.0;
                        if result.1 {
                            self.vx[0xF_usize] = 0;
                        } else {
                            self.vx[0xF_usize] = 1;
                        }
                        self.pc += 2;
                    }
                    6 => {
                        let lsb = self.vx[y as usize] & 1;
                        self.vx[x as usize] >>= 1;
                        self.vx[0xF_usize] = lsb;
                        self.pc += 2;
                    }
                    7 => {
                        let vx = self.vx[x as usize];
                        let vy = self.vx[y as usize];
                        let result = vy.overflowing_sub(vx);
                        self.vx[x as usize] = result.0;
                        if result.1 {
                            self.vx[0xF_usize] = 0;
                        } else {
                            self.vx[0xF_usize] = 1;
                        }
                        self.pc += 2;
                    }
                    0xE => {
                        let msb = self.vx[x as usize] >> 7 & 0x1;
                        self.vx[x as usize] <<= 1;
                        self.vx[0xF_usize] = msb;
                        self.pc += 2;
                    }
                    _ => {
                        unreachable!();
                    }
                }

            }
            0x9 => {
                if self.vx[x as usize] != self.vx[y as usize] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0xA => {
                self.i = nnn;
                self.pc +=2;
            }
            0xC => {
                // TODO: store the rng generator
                let mut rng = rand::thread_rng();
                let r: u8 = rng.gen();

                self.vx[x as usize] = r & nn;
                    self.pc += 2
            }
            0xD => {
                self.draw_sprite(self.vx[x as usize], self.vx[y as usize], n);
                self.pc +=2;
            }
            0xE => {
                // TODO: actually check if something is pressed down
                match nn {
                    0x9E => {
                        if self.keypad.is_key_pressed(self.vx[x as usize]) {
                            self.pc += 4;
                        } else{
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        if self.keypad.is_key_pressed(self.vx[x as usize]) {
                            self.pc += 2;
                        } else{
                            self.pc += 4;
                        }
                    }
                    _ => {
                        panic!("Unexpected 0xE instruction!!!");
                    }
                }
            }

            0xF => {
                match nn {
                    0x07 => {
                        self.vx[x as usize] = self.delay_timer;
                        self.pc += 2;
                    }
                    0x15 => {
                        self.delay_timer = self.vx[x as usize];
                        self.pc += 2;
                    }
                    0x18 => {
                        self.sound_timer = self.vx[x as usize];
                        self.pc += 2;
                    }
                    0x29 => {
                        self.i = (self.vx[x as usize] * 5) as u16;
                        self.pc += 2;
                    }
                    0x33 => {
                        let vx = self.vx[x as usize];
                        self.memory[(self.i) as usize] = vx / 100;
                        self.memory[(self.i + 1) as usize] = vx % 100 / 10;
                        self.memory[(self.i + 2) as usize] = vx % 10;
                        self.pc += 2;
                    }
                    0x55 => {
                        for ri in 0..x + 1 {
                            self.memory[(self.i + ri as u16) as usize] = self.vx[ri as usize];
                        }
                        self.pc += 2;
                    }
                    0x65 => {
                        for i in 0..x + 1 {
                            self.vx[i as usize] = self.memory[(self.i + i as u16) as usize];
                        }
                        self.i += x as u16 + 1;
                        self.pc += 2;
                    }
                    0x1E => {
                        self.i += self.vx[x as usize] as u16;
                        self.pc += 2
                    }
                    _ => {
                        unreachable!("Unrecognized instruction {:#x}", instruction);
                    }
                }
            }

            _ => {
                unreachable!("Unrecognized instruction {:#x}", instruction);
            }
        }
        if self.delay_timer > 0 {
            self.delay_timer -= 2;
        }

        if self.sound_timer > 0 {
            println!("st: {:?}", self.sound_timer);
            self.sound_timer -= 2;
        }
    }

    fn get_display_buffer(&mut self) -> [u8; HEIGHT * WIDTH] {
        self.screen.screen
    }

    fn draw_sprite(&mut self, x: u8, y: u8, height: u8) {
        self.draw = true;
        let mut erased = false;
        for yp in 0..height {
            erased = self.screen.draw(self.memory[(self.i + yp as u16) as usize], x, y + yp);
        }
        if erased {
            self.vx[0xF] = 1;
        } else {
            self.vx[0xF] = 0;
        }
    }
}

use std::fs::File;
use std::io::Read;
// use std::{thread, time};
// use std::time::{Duration, Instant};

fn main() {
    let mut file = File::open("../roms/keypad.ch8").unwrap();
    let mut data = Vec::<u8>::new();
    match file.read_to_end(&mut data) {
        Ok(_size) => {  }
        Err(err) => { panic!("failed to read file {}", err); }
    }

    let mut chip8 = Chip8::new();

    let width = 640;
    let height = 320;

    //ARGB buffer
    let mut buffer: Vec<u32> = vec![0; width * height];

    let mut window = Window::new(
        "Rust Chip8 emulator",
        width,
        height,
        WindowOptions::default(),
    ).unwrap_or_else(|e| {
        panic!("Window creation failed: {:?}", e);
    });

    chip8.load_program(data);

    // Limit to max ~60 fps update rate
    // window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if chip8.draw == false {
            let keys = window.get_keys();
            chip8.keypad.update_keys(keys);
            chip8.step();
        }

        
        let chip8_buffer = chip8.get_display_buffer();

        if chip8.draw {
            chip8.draw = false;
            for y in 0..height {
                let y_coord = y / 10;
                let offset = y * width;
                for x in 0..width {
                    let index = Display::get_index_from_coords(x / 10, y_coord);
                    let pixel = chip8_buffer[index];
                    if chip8.sound_timer > 0 {
                        let color_pixel = match pixel {
                            0 => 0x0,
                            1 => 0xffff0f,
                            _ => unreachable!(),
                        };
                        buffer[offset + x] = color_pixel;
                    } else {
                        let color_pixel = match pixel {
                            0 => 0x0,
                            1 => 0xfffff0,
                            _ => unreachable!(),
                        };
                        buffer[offset + x] = color_pixel;
                    }
                }
            }
        }
        match window.update_with_buffer(&buffer, width, height) {
            Ok(_) => {}
            Err(err) => { panic!("failed to update window {}", err); }
        }
    }
}
