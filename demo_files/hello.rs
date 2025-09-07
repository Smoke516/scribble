fn main() {
    println!("Hello, World!");
    
    let numbers = vec![1, 2, 3, 4, 5];
    for (index, number) in numbers.iter().enumerate() {
        println!("Item {}: {}", index, number);
    }
    
    let result = calculate_sum(&numbers);
    println!("Sum: {}", result);
}

fn calculate_sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_sum() {
        let numbers = vec![1, 2, 3, 4, 5];
        assert_eq!(calculate_sum(&numbers), 15);
    }
}
