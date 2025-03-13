.data
.text
.globl main
main:
    addi sp, sp, -16
    sw ra, 12(sp)
    sw s0, 0(sp)
    addi s0, sp, 0
    li t0, 5
    sw t0, -4(s0)
    li t1, 10
    sw t1, -8(s0)
    lw t2, -4(s0)
    lw t3, -8(s0)
    add t2, t2, t3
    sw t2, -12(s0)
    lw t4, -12(s0)
    mv a0, t4
    lw ra, 12(sp)
    lw s0, 0(sp)
    addi sp, sp, 16
    ret
