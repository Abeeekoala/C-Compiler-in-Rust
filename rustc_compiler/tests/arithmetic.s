.data
.text
    .text
    .globl main
main:
    addi sp, sp, -16
    sw ra, 12(sp)
    sw s0, 0(sp)
    addi s0, sp, 0
    li t0, 5
    li t1, 10
    mul t0, t0, t1
    li t2, 3
    add t0, t0, t2
    mv a0, t0
    lw ra, 12(sp)
    lw s0, 0(sp)
    addi sp, sp, 16
    ret
    lw ra, 12(sp)
    lw s0, 0(sp)
    addi sp, sp, 16
    ret
