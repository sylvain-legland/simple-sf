// Ref: FT-SSF-022
// Genetic Algorithm for workflow optimization

#[derive(Clone, Debug)]
pub struct Individual {
    pub genes: Vec<String>,
    pub fitness: f64,
}

pub struct GeneticOptimizer {
    pub population: Vec<Individual>,
    pub generation: usize,
}

impl GeneticOptimizer {
    pub fn new(pop_size: usize, gene_pool: Vec<String>) -> Self {
        let gene_len = gene_pool.len();
        let population = (0..pop_size)
            .map(|i| {
                let genes: Vec<String> = (0..gene_len)
                    .map(|j| gene_pool[(i + j * 7) % gene_len].clone())
                    .collect();
                Individual { genes, fitness: 0.0 }
            })
            .collect();
        Self { population, generation: 0 }
    }

    pub fn set_fitness(&mut self, idx: usize, score: f64) {
        if let Some(ind) = self.population.get_mut(idx) {
            ind.fitness = score;
        }
    }

    pub fn evolve(&mut self) {
        self.population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        let half = self.population.len() / 2;
        let survivors: Vec<Individual> = self.population[..half].to_vec();
        let mut next_gen = survivors.clone();

        // Crossover: pair consecutive survivors
        for pair in survivors.chunks(2) {
            if pair.len() == 2 {
                let point = pair[0].genes.len() / 2;
                let mut child_genes = pair[0].genes[..point].to_vec();
                child_genes.extend_from_slice(&pair[1].genes[point..]);
                next_gen.push(Individual { genes: child_genes, fitness: 0.0 });
            }
        }

        // Mutation: swap two genes in ~10% of individuals
        let seed = self.generation;
        for (i, ind) in next_gen.iter_mut().enumerate() {
            if (seed + i) % 10 == 0 && ind.genes.len() >= 2 {
                let a = (seed + i) % ind.genes.len();
                let b = (seed + i + 3) % ind.genes.len();
                ind.genes.swap(a, b);
            }
        }

        // Fill remaining slots by cloning best
        while next_gen.len() < self.population.len() {
            next_gen.push(survivors[next_gen.len() % survivors.len()].clone());
        }
        next_gen.truncate(self.population.len());

        self.population = next_gen;
        self.generation += 1;
    }

    pub fn best(&self) -> &Individual {
        self.population
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
            .expect("population must not be empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_population() {
        let pool = vec!["a".into(), "b".into(), "c".into()];
        let ga = GeneticOptimizer::new(10, pool);
        assert_eq!(ga.population.len(), 10);
        assert_eq!(ga.generation, 0);
    }

    #[test]
    fn evolve_increments_generation() {
        let pool = vec!["x".into(), "y".into()];
        let mut ga = GeneticOptimizer::new(6, pool);
        ga.evolve();
        assert_eq!(ga.generation, 1);
        ga.evolve();
        assert_eq!(ga.generation, 2);
    }

    #[test]
    fn best_returns_highest_fitness() {
        let pool = vec!["a".into(), "b".into()];
        let mut ga = GeneticOptimizer::new(4, pool);
        ga.set_fitness(0, 0.3);
        ga.set_fitness(1, 0.9);
        ga.set_fitness(2, 0.1);
        assert!((ga.best().fitness - 0.9).abs() < f64::EPSILON);
    }
}
