#include <SDL2/SDL.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <unistd.h>

#define SCREEN_WIDTH 64
#define SCREEN_HEIGHT 32
#define SCALE_FACTOR 10

// last nibble
#define N(INSTR) (INSTR & 0x00F)

// last two nibbles
#define NN(INSTR) (INSTR & 0x0FF)
// last 3 nibbles
#define NNN(INSTR) (INSTR & 0x0FFF)

#define X(INSTR) ((INSTR & 0x0F00) >> 8)
#define Y(INSTR) ((INSTR & 0x00F0) >> 4)

typedef struct chip {
    uint8_t await_key; // TODO: bool
    uint8_t await_register;
    uint8_t draw_screen;
    
    uint16_t pc;
    uint16_t i; // special address register
    uint8_t registers[16]; // registers TODO: rename to vx
    uint8_t si; // stack index
    uint16_t stack[16];
    uint8_t sound_timer;
    uint8_t delay_timer;
    uint8_t memory[4096];
    uint32_t pixels[8192];
    uint8_t keys[16]; // key flags
} chip_t;

char font[] = {
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
};

void
write_font(chip_t *chip, uint16_t offset) {
    for (uint16_t i = 0; i < 80; i++) {
        chip->memory[offset + i] = font[i];
    }
}

void
chip_init(chip_t *chip) {
    chip->await_key = 0;
    chip->draw_screen = 0;
    chip->await_register = 0;
    chip->pc = 0x200;
    chip->si = 0;
    chip->sound_timer = 0;
    chip->delay_timer = 0;

    memset(chip->registers, 0, 16 * sizeof(uint8_t));
    memset(chip->keys, 0, 16 * sizeof(uint8_t));
    
    write_font(chip, 0x050);
}

uint8_t key_to_chip(SDL_Event *e) {
    switch(e->key.keysym.sym){
    case SDLK_1:
        return 1;
    case SDLK_2:
        return 2;
    case SDLK_3:
        return 3;
    case SDLK_4:
        return 0xC;

    case SDLK_q:
        return 4;
    case SDLK_w:
        return 5;
    case SDLK_e:
        return 6;
    case SDLK_r:
        return 0xD;

    case SDLK_a:
        return 7;
    case SDLK_s:
        return 8;
    case SDLK_d:
        return 9;
    case SDLK_f:
        return 0xE;

    case SDLK_z:
        return 0xA;
    case SDLK_x:
        return 0;
    case SDLK_c:
        return 0xB;
    case SDLK_v:
        return 0xF;
        
    default:
        return 0;
    }
}

void
load(chip_t *chip, FILE *file) {
    uint8_t * program_memory = &chip->memory[0x200];
    fread(program_memory, sizeof(uint16_t), ((4096 / 2 - 0x200)), file);
}

void
step(chip_t *chip) {
    uint16_t instr = (chip->memory[chip->pc] << 8) | chip->memory[chip->pc + 1];
    chip->pc += 2;
    
    // printf("pc %i | instr: %04X\n", chip->pc-1, instr);
    
    // first nibble for instruction type
    uint8_t nib = (instr & 0xF000) >> 12;
    switch (nib) {
    case 0x0:
        if (instr == 0x00E0) {
            // clear screen
            chip->draw_screen = 1;
            memset(chip->pixels, 0, SCREEN_HEIGHT * SCREEN_WIDTH * sizeof(uint32_t));
        } else if (instr == 0x00EE) {
            // return
            chip->si--;
            uint16_t ret_pc = chip->stack[chip->si];
            chip->pc = ret_pc;
        } else {
            // call machine code NNN, not used in emulation
        }
        break;
    case 0x1:
        // goto
        chip->pc = NNN(instr);
        break;
    case 0x2:
        // goto subrutine
        chip->stack[chip->si] = chip->pc;
        chip->si++;
        chip->pc = NNN(instr);
        break;
    case 0x3:
        // skip if Vx == NN
        if (chip->registers[X(instr)] == NN(instr)) {
            chip->pc += 2;
        }
        break;
    case 0x4:
        // Vx != NN
        if (chip->registers[X(instr)] != NN(instr)) {
            chip->pc += 2;
        }
        break;
    case 0x5:
        // Vx == Vy
        if (chip->registers[X(instr)] == chip->registers[Y(instr)]) {
            chip->pc += 2;
        }
        break;
    case 0x6:
        // Vx = NN
        chip->registers[X(instr)] = NN(instr);
        break;
    case 0x7:
        // Vx += NN
        chip->registers[X(instr)] += NN(instr);
        break;
    case 0x8:
        // register manipulation
        switch (N(instr)) {
        case 0x0:
            chip->registers[X(instr)] = chip->registers[Y(instr)];
            break;
        case 0x1:
            chip->registers[X(instr)] |= chip->registers[Y(instr)];
            break;
        case 0x2:
            chip->registers[X(instr)] &= chip->registers[Y(instr)];
            break;
        case 0x3:
            chip->registers[X(instr)] ^= chip->registers[Y(instr)];
            break;
        case 0x4: {
            uint8_t will_carry;
            if (chip->registers[X(instr)] > UINT8_MAX - chip->registers[Y(instr)]) {
                will_carry = 1;
            } else {
                will_carry = 0;
            }
            chip->registers[X(instr)] += chip->registers[Y(instr)];
            chip->registers[0xF] = will_carry;
            break;
        }
        case 0x5: {
            uint8_t will_borrow;
            if (chip->registers[X(instr)] < chip->registers[Y(instr)]) {
                will_borrow = 0;
            } else {
                will_borrow = 1;
            }
            chip->registers[X(instr)] -= chip->registers[Y(instr)];
            chip->registers[0xF] = will_borrow;
            break;
        }
        case 0x6: {
            uint8_t lsb = chip->registers[X(instr)] & 1;
            chip->registers[X(instr)] >>= 1;
            chip->registers[0xF] = lsb;
            break;
        }
        case 0x7: {
            uint8_t will_borrow;
            if (chip->registers[X(instr)] > chip->registers[Y(instr)]) {
                will_borrow = 0;
            } else {
                will_borrow = 1;
            }
            
            chip->registers[X(instr)] = chip->registers[Y(instr)] - chip->registers[X(instr)];
            chip->registers[0xF] = will_borrow;
            break;
        }
        case 0xE: {
            uint8_t msb = (chip->registers[X(instr)] >> 7) & 0x1;;
            chip->registers[X(instr)] <<= 1;
            chip->registers[0xF] = msb;
            break;
        }
        }
        break;
    case 0x9:
        // Vx != Vy
        if (chip->registers[X(instr)] != chip->registers[Y(instr)]) {
            chip->pc += 2;
        }
        break;
    case 0xA:
        chip->i = NNN(instr);
        break;
    case 0xB:
        // jump to v0 + NNN
        chip->pc = chip->registers[0] + NNN(instr);
        break;
    case 0xC:
        // Vx = rand() & NN
        chip->registers[X(instr)] = rand() & NN(instr);
        break;
    case 0xD: {
        // draw px
        chip->draw_screen = 1;
        uint8_t base_x = chip->registers[X(instr)] % SCREEN_WIDTH;
        uint8_t base_y = chip->registers[Y(instr)] % SCREEN_HEIGHT;
        
        for (uint8_t height = 0; height < N(instr); height++) {
            uint8_t byte = chip->memory[chip->i + height];
            for (uint8_t w = 0; w < 8; w++) {
                uint8_t x = base_x + w;
                uint8_t y = base_y + height;
                if (byte & 1 << (7 - w) && x < SCREEN_WIDTH && y < SCREEN_HEIGHT) {
                    
                    uint16_t index = (y * SCREEN_WIDTH) + x;

                    if (chip->pixels[index] == 255) {
                        chip->pixels[index] = 0;
                    } else {
                        chip->pixels[index] = 255;
                    }
                }
            }
        }
        break;
    }
    case 0xE: {
        switch (NN(instr)) {
        case 0x9E:
            if (chip->keys[chip->registers[X(instr)]]) {
                chip->pc += 2;
            }
            break;
        case 0xA1:
            if (!chip->keys[chip->registers[X(instr)]]) {
                chip->pc += 2;
            }
            break;
        }
        break;
    }
    case 0xF:
        switch (NN(instr)) {
        case 0x07:
            chip->registers[X(instr)] = chip->delay_timer;
            break;
        case 0x0A:
            chip->await_key = 1;
            chip->await_register = X(instr);
            break;
        case 0x15:
            chip->delay_timer = chip->registers[X(instr)];
            break;
        case 0x18:
            chip->sound_timer = chip->registers[X(instr)];
            break;
        case 0x1E:
            // no carry
            chip->i += chip->registers[X(instr)];
            break;
        case 0x29:
            chip->i = 0x050 + X(instr);
            break;
        case 0x33: {
            uint8_t vx = chip->registers[X(instr)];
            
            chip->memory[chip->i + 0] = vx / 100;
            chip->memory[chip->i + 1] = (vx / 10) % 10;
            chip->memory[chip->i + 2] = (vx % 10);
            break;
        }
        case 0x55:
            // reg_store
            for (uint8_t offset = 0; offset <= X(instr); offset++) {
                chip->memory[chip->i + offset] = chip->registers[offset];
            }
            break;
        case 0x65:
            // reg_load
            for (uint8_t offset = 0; offset <= X(instr); offset++) {
                chip->registers[offset] = chip->memory[chip->i + offset];
            }
            break;
        }
        break;
    }
}

int main() {
    chip_t chip;
    chip_init(&chip);

    FILE * stream = fopen("../roms/corax.ch8", "r");

    load(&chip, stream);

    fclose(stream);
    
    SDL_Init(SDL_INIT_VIDEO);

    SDL_Window * window = SDL_CreateWindow("chip-8 emu",
                                           SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED, SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR, 0);

    SDL_Renderer * renderer = SDL_CreateRenderer(window, -1, 0);

    SDL_Texture * texture = SDL_CreateTexture(renderer, SDL_PIXELFORMAT_ARGB8888, SDL_TEXTUREACCESS_STATIC, SCREEN_WIDTH * SCALE_FACTOR, SCREEN_HEIGHT * SCALE_FACTOR);

    SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
    
    memset(chip.pixels, 0, SCREEN_WIDTH * SCREEN_HEIGHT * sizeof(uint32_t));

    SDL_Scancode keymappings[16] = {
        SDL_SCANCODE_X,
        SDL_SCANCODE_1, SDL_SCANCODE_2, SDL_SCANCODE_3,
        SDL_SCANCODE_Q, SDL_SCANCODE_W, SDL_SCANCODE_E,
        SDL_SCANCODE_A, SDL_SCANCODE_S, SDL_SCANCODE_D,
        SDL_SCANCODE_Z, SDL_SCANCODE_C,
        SDL_SCANCODE_4, SDL_SCANCODE_R, SDL_SCANCODE_F, SDL_SCANCODE_V,
    };
    
    while(1) {
        SDL_Event e;
        while(SDL_PollEvent(&e) > 0 || chip.await_key) {
            const Uint8* state = SDL_GetKeyboardState(NULL);
            switch(e.type) {
            case SDL_QUIT:
                return 0;
                break;

            default: {
                if (state[SDL_SCANCODE_ESCAPE]) {
                    return 0;
                }

                for (int keycode = 0; keycode < 16; keycode++) {
                    if (state[keymappings[keycode]]) {
                        chip.registers[chip.await_register] = state[keymappings[keycode]];
                        chip.await_key = 0;
                    }
                    chip.keys[keycode] = state[keymappings[keycode]];
                }
                if (chip.await_key == 1) {
                    if (chip.delay_timer > 0) {
                        chip.delay_timer--;
                    }
                    SDL_Delay(1);
                }
                break;
            }
            }
        }        
        step(&chip);

        if (chip.draw_screen || 1) {
            SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
            SDL_RenderClear(renderer);
            SDL_SetRenderDrawColor(renderer, 255, 255, 255, 255);
            for (uint16_t y = 0; y < SCREEN_HEIGHT; y++) {
                for (uint16_t x = 0; x < SCREEN_WIDTH; x++) {
                    if (chip.pixels[y*SCREEN_WIDTH + x]) {
                        SDL_Rect rect = { x * SCALE_FACTOR, y * SCALE_FACTOR, SCALE_FACTOR, SCALE_FACTOR };
                        SDL_RenderFillRect(renderer, &rect);
                    }
                }
            }
            SDL_RenderPresent(renderer);

            chip.draw_screen = 0;
        }

        if (chip.delay_timer > 0) {
            chip.delay_timer--;
        }
        SDL_Delay(1);
    }

    SDL_DestroyTexture(texture);
    SDL_DestroyRenderer(renderer);
    SDL_DestroyWindow(window);
    SDL_Quit();
    
    return EXIT_SUCCESS;
}
