
// -------------------------------------------------------------------------------------------------------------------
// no external packages imported
// https://en.wikipedia.org/wiki/Xorshift#xorshiftr+

pub struct PRNG
{
    state: [u64; 2],
}

impl Default for PRNG
{
    fn default() -> Self
    {
        return Self {
            state: [ // generated with random.org/bytes
                0x_63_FE_C5_30_B8_C3_07_95, 
                0x_0B_11_1B_51_15_83_A7_A2
            ]
        };
    }
}

impl PRNG
{
    fn get_next_number(&mut self) -> u64
    {
        let mut x = self.state[0];
        let y = self.state[1];

        self.state[0] = y;

        x ^= x << 23;
        x ^= x >> 17;
        x ^= y;

        self.state[1] = x.wrapping_add(y); // can't use + because it panics if there is an overflow
        return x;
    }

    pub fn rand_01(&mut self) -> f64
    {
        return (self.get_next_number() as f64) / (u64::MAX as f64);
    }
}
