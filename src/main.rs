use rand::Rng;
use std::{env, process, thread, time};
use std::{fs::File, io::Read};

struct CPU {
    register_i: u16,
    registers: [u8; 16],
    position_in_memory: usize,
    memory: [u8; 0x1000],

    stacks: [u16; 16],
    stack_pointer: usize,

    delayed_timer: u8,
    sound_timer: u8,

    monitor: [[u8; 64]; 32],
}

impl CPU {
    fn read_opcode(&self) -> u16 {
        let op1 = self.memory[self.position_in_memory] as u16;
        let op2 = self.memory[self.position_in_memory + 1] as u16;

        op1 << 8 | op2
    }

    fn run(&mut self) {
        let mut rng = rand::thread_rng();

        loop {
            let opcode = self.read_opcode();

            // let c = ((opcode & 0xF000) >> 12) as u8;
            let x = ((opcode & 0x0F00) >> 8) as u8;
            let y = ((opcode & 0x00F0) >> 4) as u8;
            let n = ((opcode & 0x000F) >> 0) as u8;

            let nnn = (opcode & 0x0FFF) as usize;
            let kk = (opcode & 0x0FF) as u8;

            // println!(
            //     "{:04x} - {} - {:?} - {}",
            //     opcode, self.position_in_memory, self.registers, self.register_i,
            // );
            self.position_in_memory += 2;
            match opcode {
                0x0000 => {
                    return;
                }
                // 00E0 - CLS
                0x00E0 => {
                    self.monitor = [[0; 64]; 32];
                }
                // 00EE - RET
                0x00EE => {
                    self.stack_pointer -= 1;
                    self.position_in_memory = self.stacks[self.stack_pointer] as usize;
                }
                // 0nnn - SYS addr
                // 0x0000..=0x0FFF => {}
                // 1nnn - JP addr
                0x1000..=0x1FFF => {
                    self.position_in_memory = nnn;
                }
                // 2nnn - CALL addr
                0x2000..=0x2FFF => {
                    self.stacks[self.stack_pointer] = self.position_in_memory as u16;
                    self.stack_pointer += 1;
                    self.position_in_memory = nnn;
                }
                // 3xkk - SE Vx, byte
                0x3000..=0x3FFF => {
                    self.se(self.registers[x as usize], kk);
                }
                // 4xkk - SNE Vx, byte
                0x4000..=0x4FFF => {
                    self.sne(self.registers[x as usize], kk);
                }
                // 5xy0 - SE Vx, Vy
                0x5000..=0x5FF0 => {
                    self.se(self.registers[x as usize], self.registers[y as usize]);
                }
                // 6xkk - LD Vx, byte
                0x6000..=0x6FFF => {
                    self.registers[x as usize] = kk;
                }
                // 7xkk - ADD Vx, byte
                0x7000..=0x7FFF => {
                    self.registers[x as usize] =
                        (self.registers[x as usize] as u16 + kk as u16) as u8;
                }
                0x8000..=0x8FFF => match n {
                    // 8xy0 - LD Vx, Vy
                    0 => {
                        self.registers[x as usize] = self.registers[y as usize];
                    }
                    // 8xy1 - OR Vx, Vy
                    1 => {
                        self.registers[x as usize] |= self.registers[y as usize];
                    }
                    // 8xy2 - AND Vx, Vy
                    2 => {
                        self.registers[x as usize] &= self.registers[y as usize];
                    }
                    // 8xy3 - XOR Vx, Vy
                    3 => {
                        self.registers[x as usize] ^= self.registers[y as usize];
                    }
                    // 8xy4 - ADD Vx, Vy
                    4 => {
                        let (s, over) =
                            self.registers[x as usize].overflowing_add(self.registers[y as usize]);
                        self.registers[x as usize] = s;
                        self.registers[0xF] = if over { 1 } else { 0 };
                    }
                    // 8xy5 - SUB Vx, Vy
                    5 => {
                        self.registers[0xF] =
                            if self.registers[x as usize] > self.registers[y as usize] {
                                1
                            } else {
                                0
                            };
                        self.registers[x as usize] =
                            self.registers[x as usize].wrapping_sub(self.registers[y as usize]);
                    }
                    // 8xy6 - SHR Vx {, Vy}
                    6 => {
                        self.registers[0xF] = self.registers[x as usize] & 1;
                        self.registers[x as usize] >>= 1;
                    }
                    // 8xy7 - SUBN Vx, Vy
                    7 => {
                        self.registers[0xF] =
                            if self.registers[y as usize] > self.registers[x as usize] {
                                1
                            } else {
                                0
                            };
                        self.registers[x as usize] =
                            self.registers[y as usize].wrapping_sub(self.registers[x as usize]);
                    }
                    // 8xyE - SHL Vx {, Vy}
                    0xE => {
                        self.registers[0xF] = if self.registers[x as usize] >> 7 == 1 {
                            1
                        } else {
                            0
                        };
                        self.registers[x as usize] <<= 1;
                    }
                    _ => panic!("unknow opcode {:04x}", opcode),
                },
                // 9xy0 - SNE Vx, Vy
                0x9000..=0x9FF0 => {
                    self.sne(self.registers[x as usize], self.registers[y as usize]);
                }
                // Annn - LD I, addr
                0xA000..=0xAFFF => {
                    self.register_i = nnn as u16;
                }
                // Bnnn - JP V0, addr
                0xB000..=0xBFFF => {
                    self.position_in_memory = nnn as usize + self.registers[0] as usize;
                }
                // Cxkk - RND Vx, byte
                0xC000..=0xCFFF => {
                    let r: u8 = rng.gen();
                    self.registers[x as usize] = r & kk;
                    // panic!("")
                }
                // Dxyn - DRW Vx, Vy, nibble
                0xD000..=0xDFFF => {
                    self.registers[0xF] = 0;
                    let v_x = self.registers[x as usize] as usize;
                    let v_y = self.registers[y as usize] as usize;
                    for i in 0..n {
                        let l_y = (v_y + i as usize) % 32;
                        let sprite = self.memory[self.register_i as usize + i as usize];
                        for j in 0..8 {
                            let l_x = (v_x + j as usize) % 64;
                            let color = if sprite & (1 << (7 - j)) > 0 { 1 } else { 0 };
                            self.registers[0xF] |= if color == self.monitor[l_y][l_x] {
                                1
                            } else {
                                0
                            };
                            self.monitor[l_y][l_x] ^= color;
                        }
                    }

                    for row in self.monitor {
                        for col in row {
                            if col == 1 {
                                print!("{}", "*");
                            } else {
                                print!("{}", "_");
                            }
                        }
                        println!("");
                    }
                    println!("");
                    println!("");
                }
                // Ex9E - SKP Vx
                0xE09E..=0xEF9E => {
                    // todo!("if key Vx is pressed: {}", opcode);
                }
                // ExA1 - SKNP Vx
                0xE0A1..=0xEFA1 => {
                    self.position_in_memory += 2;
                    // todo!("if key Vx is not pressed: {}", opcode);
                }
                0xF000..=0xFFFF => match (y, n) {
                    // Fx07 - LD Vx, DT
                    (0, 7) => {
                        self.registers[x as usize] = self.delayed_timer;
                    }
                    // Fx0A - LD Vx, K
                    (0, 0xA) => {
                        todo!("if key is pressed, store to: {}", x);
                    }
                    // Fx15 - LD DT, Vx
                    (1, 5) => {
                        self.delayed_timer = self.registers[x as usize];
                    }
                    // Fx18 - LD ST, Vx
                    (1, 8) => {
                        self.sound_timer = self.registers[x as usize];
                    }
                    // Fx1E - ADD I, Vx
                    (1, 0xE) => {
                        self.register_i += self.registers[x as usize] as u16;
                    }
                    // Fx29 - LD F, Vx
                    (2, 9) => {
                        self.register_i = self.registers[x as usize] as u16 * 5;
                    }
                    // Fx33 - LD B, Vx
                    (3, 3) => {
                        self.memory[self.register_i as usize + 0] =
                            self.registers[x as usize] / 100;
                        self.memory[self.register_i as usize + 1] =
                            self.registers[x as usize] % 100 / 10;
                        self.memory[self.register_i as usize + 2] = self.registers[x as usize] % 10;
                    }
                    // Fx55 - LD [I], Vx
                    (5, 5) => {
                        for i in 0..=x {
                            self.memory[self.register_i as usize + i as usize] =
                                self.registers[i as usize];
                        }
                    }
                    // Fx65 - LD Vx, [I]
                    (6, 5) => {
                        for i in 0..=x {
                            self.registers[i as usize] =
                                self.memory[self.register_i as usize + i as usize];
                        }
                    }
                    _ => panic!("unknow opcode {:04x}", opcode),
                },
                _ => panic!("unknow opcode {:04x}", opcode),
            }

            if self.delayed_timer > 0 {
                self.delayed_timer -= 1;
            }

            if self.sound_timer > 0 {
                self.sound_timer -= 1;
            }

            // thread::sleep(time::Duration::from_secs_f32(1.0 / 120.0));
        }
    }

    fn se(&mut self, x: u8, y: u8) {
        if x == y {
            self.position_in_memory += 2;
        }
    }

    fn sne(&mut self, x: u8, y: u8) {
        if x != y {
            self.position_in_memory += 2;
        }
    }
}

fn main() {
    let mut cpu = CPU {
        register_i: 0,

        sound_timer: 0,
        delayed_timer: 0,

        registers: [0; 16],
        position_in_memory: 0,
        memory: [0; 0x1000],

        stacks: [0; 16],
        stack_pointer: 0,

        monitor: [[0; 64]; 32],
    };

    let fonts: [u8; 80] = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0,
        0xF0, 0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0,
        0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0,
        0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0,
        0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0,
        0xF0, 0x80, 0xF0, 0x80, 0x80,
    ];

    cpu.position_in_memory = 512;

    cpu.registers = [0; 16];

    cpu.memory = [0; 0x1000];
    cpu.memory[..fonts.len()].copy_from_slice(&fonts);

    // cpu.memory[512] = 0xD0;
    // cpu.memory[513] = 0x05;
    // cpu.memory[514] = 0xD1;
    // cpu.memory[515] = 0x05;

    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: chip8 <rom>");
        process::exit(1);
    }

    let mut f = File::open(&args[1]).unwrap();

    let mut rom = [0u8; 3584];
    f.read(&mut rom).unwrap();

    cpu.memory[512..].copy_from_slice(&rom);

    cpu.run();
}
