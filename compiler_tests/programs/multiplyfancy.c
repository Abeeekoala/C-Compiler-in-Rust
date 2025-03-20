float multiply(float x, float y)
{
    float acc=0.0;
    if(x < 0){
        return -multiply(-x, y);
    }

    while(x > 0){
        acc += y;
        x--;
    }
    return acc;
}
