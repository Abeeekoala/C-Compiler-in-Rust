.text
.globl f
f:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    addi sp, sp, -32
    li t0, 13
    # Assign to struct member z.y
    sw t0, -36(s0)
    # Access struct member z.y
    lw t0, -36(s0)
    mv a0, t0
    mv sp, s0
    addi sp, sp, -32
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
