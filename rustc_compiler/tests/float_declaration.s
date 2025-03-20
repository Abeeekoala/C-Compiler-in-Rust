.data
.float_const0:
    .word 0x3f800000  # float 1
.float_const1:
    .word 0x3f800000  # float 1

.text
.globl main
main:
    addi sp, sp, -32
    sw ra, 28(sp)
    sw s0, 24(sp)
    addi s0, sp, 32
    sw s1, 20(sp)
    sw s2, 16(sp)
    addi sp, sp, -32
    la t0, .float_const0
    flw ft0, 0(t0)
    fsw ft0, -36(s0)
    flw ft0, -36(s0)
    la t0, .float_const1
    flw ft1, 0(t0)
    fadd.s ft0, ft0, ft1
    sw ft0, -36(s0)
    flw ft1, -36(s0)
    fmv.s fa0, ft1
    mv sp, s0
    addi sp, sp, -32
    lw s2, 16(sp)
    lw s1, 20(sp)
    lw s0, 24(sp)
    lw ra, 28(sp)
    addi sp, sp, 32
    ret
