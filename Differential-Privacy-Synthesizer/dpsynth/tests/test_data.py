"""Tests for the utils/data module: DataPreprocessor and dataset loaders."""

import csv
import os
import tempfile

import numpy as np
import pytest

from dpsynth.utils.data import DataPreprocessor, load_benchmark_dataset


class TestDataPreprocessor:
    """Test the DataPreprocessor class."""

    @pytest.fixture
    def mixed_data(self):
        """Create mixed-type data for testing."""
        rng = np.random.RandomState(42)
        n = 100
        data = np.column_stack([
            rng.randint(0, 3, n),   # categorical: 3 categories
            rng.randn(n),            # continuous
            rng.randint(0, 5, n),   # categorical: 5 categories
            rng.randn(n) * 10,       # continuous
        ])
        return data

    def test_fit_auto_detect(self, mixed_data):
        """Test automatic column type detection."""
        prep = DataPreprocessor()
        prep.fit(mixed_data)
        assert len(prep.categorical_columns) > 0
        assert len(prep.continuous_columns) > 0

    def test_transform_all_numerical(self, mixed_data):
        """Test that transform produces all-numerical output."""
        prep = DataPreprocessor()
        transformed = prep.fit_transform(mixed_data)
        assert transformed.shape == mixed_data.shape
        assert np.issubdtype(transformed.dtype, np.floating)
        assert not np.any(np.isnan(transformed)), "NaNs should be imputed"

    def test_fit_transform_no_missing_values(self, mixed_data):
        """Test that output has no NaN values."""
        prep = DataPreprocessor()
        transformed = prep.fit_transform(mixed_data)
        assert np.all(~np.isnan(transformed))

    def test_categorical_with_unseen(self, mixed_data):
        """Test that unseen categories are handled."""
        prep = DataPreprocessor(categorical_columns=[0, 2])
        prep.fit(mixed_data)

        # Create data with unseen category
        rng = np.random.RandomState(99)
        new_data = np.column_stack([
            np.full(10, 999).astype(float),  # unseen category
            rng.randn(10),
            np.full(10, 999).astype(float),  # unseen category
            rng.randn(10) * 10,
        ]).astype(object)

        transformed = prep.transform(new_data)
        assert transformed.shape == new_data.shape
        assert not np.any(np.isnan(transformed))

    def test_explicit_column_specification(self):
        """Test with explicit column type specification."""
        rng = np.random.RandomState(42)
        data = rng.randn(50, 5)

        prep = DataPreprocessor(
            categorical_columns=[0, 1],
            continuous_columns=[2, 3, 4],
        )
        transformed = prep.fit_transform(data)
        assert transformed.shape == (50, 5)

    def test_constant_column(self):
        """Test with a constant-valued column."""
        rng = np.random.RandomState(42)
        data = np.column_stack([
            np.ones(50),
            rng.randn(50),
        ])
        prep = DataPreprocessor(continuous_columns=[0, 1])
        transformed = prep.fit_transform(data)
        assert transformed.shape == (50, 2)
        assert not np.any(np.isnan(transformed))

    def test_transform_before_fit_raises(self):
        """Test that calling transform before fit raises."""
        prep = DataPreprocessor()
        with pytest.raises(RuntimeError):
            prep.transform(np.array([[1, 2, 3]]))

    def test_with_nan_values(self):
        """Test handling of NaN values in input."""
        data = np.array([
            [1.0, np.nan, 3.0],
            [np.nan, 5.0, 6.0],
            [7.0, 8.0, np.nan],
        ]).astype(object)

        prep = DataPreprocessor()
        transformed = prep.fit_transform(data)
        assert transformed.shape == (3, 3)
        assert not np.any(np.isnan(transformed))

    def test_dataclass_attributes(self):
        """Test the Dataset dataclass."""
        from dpsynth.utils.data import Dataset
        dataset = Dataset(
            data=np.array([[1, 2], [3, 4]]),
            target_column=1,
            name="test",
        )
        assert dataset.n_samples == 2
        assert dataset.n_features == 2
        assert dataset.name == "test"
        assert len(dataset.column_names) == 2


class TestLoadBenchmarkDataset:
    """Test benchmark dataset loading."""

    def test_unknown_dataset(self):
        """Test that unknown dataset name returns None."""
        dataset = load_benchmark_dataset("nonexistent_dataset_xyz")
        assert dataset is None

    def test_adult_dataset(self):
        """Test loading Adult dataset (may require network)."""
        dataset = load_benchmark_dataset("adult")
        if dataset is not None:
            assert dataset.name == "adult"
            assert dataset.n_features > 0
            assert dataset.n_samples > 0
            assert dataset.target_column >= 0
            assert dataset.categorical_columns is not None
            assert dataset.continuous_columns is not None

    def test_case_insensitive(self):
        """Test that dataset name matching is case-insensitive."""
        dataset = load_benchmark_dataset("ADULT")
        if dataset is not None:
            assert dataset.name == "adult"


class TestCSVDataIntegration:
    """Test loading data from a CSV file via DataPreprocessor."""

    def test_csv_roundtrip(self):
        """Test that CSV data can be loaded and preprocessed."""
        # Create a temporary CSV file
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".csv", delete=False, newline=""
        ) as f:
            writer = csv.writer(f)
            writer.writerow(["age", "income", "education", "score"])
            writer.writerow([25, 50000, "bachelors", 0.85])
            writer.writerow([30, 60000, "masters", 0.92])
            writer.writerow([35, 70000, "phd", 0.78])
            writer.writerow([40, 80000, "bachelors", 0.95])
            writer.writerow([45, 90000, "masters", 0.88])
            csv_path = f.name

        try:
            import pandas as pd
            df = pd.read_csv(csv_path)
            data = df.values

            prep = DataPreprocessor()
            transformed = prep.fit_transform(data)

            assert transformed.shape == (5, 4)
            assert np.issubdtype(transformed.dtype, np.floating)
        finally:
            os.unlink(csv_path)
