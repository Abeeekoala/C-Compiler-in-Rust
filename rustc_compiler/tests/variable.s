.text
.globl f
f:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    sw s1, 20(sp)
    sw s2, 16(sp)
    addi sp, sp, -32
    sw a0, -36(s0)
    sw a1, -40(s0)
    lw t0, -36(s0)
    mv a0, t0
    mv sp, s0
    addi sp, sp, -32
    lw s2, 16(sp)
    lw s1, 20(sp)
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
.globl main
main:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    sw s1, 20(sp)
    sw s2, 16(sp)
    addi sp, sp, -32
    li t0, 5
    sw t0, -36(s0)
    addi sp, sp, -0
    lw t0, -36(s0)
    mv a0, t0
    li a1, 10
    call f
    mv t0, a0
    addi sp, sp, 0
    sw t0, -40(s0)
    lw t0, -40(s0)
    mv a0, t0
    mv sp, s0
    addi sp, sp, -32
    lw s2, 16(sp)
    lw s1, 20(sp)
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
