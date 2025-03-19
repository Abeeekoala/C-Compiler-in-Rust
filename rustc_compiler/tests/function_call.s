.data
a:
    .word 0  # Global variable: a

.text
    li t0, 69
    sw t0, 0(s0)
.globl f
f:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw fp, 24(sp)
    addi fp, sp, 32
    sw s1, 20(sp)
    sw s2, 16(sp)
    addi sp, sp, -32
    sw a0, -36(s0)
    sw a1, -40(s0)
    li t0, 10
    sw t0, -44(s0)
    li t0, 20
    sw t0, -48(s0)
    li t0, 30
    sw t0, -52(s0)
    mv sp, fp
    addi sp, sp, -32
    lw s2, 16(sp)
    lw s1, 20(sp)
    lw fp, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
.globl main
main:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw fp, 24(sp)
    addi fp, sp, 32
    sw s1, 20(sp)
    sw s2, 16(sp)
    addi sp, sp, -32
    li t0, 5
    sw t0, -36(s0)
    li t0, 2
    sw t0, -40(s0)
    addi sp, sp, -0
    lw t0, -36(s0)
    mv a0, t0
    lw t0, -40(s0)
    mv a1, t0
    call f
    mv t0, a0
    addi sp, sp, 0
    li t0, 3
    sw t0, -44(s0)
    lw t0, -40(s0)
    mv a0, t0
    mv sp, fp
    addi sp, sp, -32
    lw s2, 16(sp)
    lw s1, 20(sp)
    lw fp, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
