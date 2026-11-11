"""Data loading and preprocessing utilities for benchmark datasets."""

from __future__ import annotations

import logging
from dataclasses import dataclass

import numpy as np
import pandas as pd
from sklearn.preprocessing import LabelEncoder, StandardScaler

logger = logging.getLogger(__name__)


@dataclass
class Dataset:
    """A loaded and preprocessed dataset.

    Attributes:
        data: Full dataset as numpy array.
        target_column: Index of the target column (or -1 if none).
        column_names: List of column names.
        categorical_columns: List of indices of categorical columns.
        continuous_columns: List of indices of continuous columns.
        name: Dataset name.
    """

    data: np.ndarray
    target_column: int = -1
    column_names: list[str] | None = None
    categorical_columns: list[int] | None = None
    continuous_columns: list[int] | None = None
    name: str = ""

    def __post_init__(self):
        if self.column_names is None:
            self.column_names = [f"col_{i}" for i in range(self.data.shape[1])]

    @property
    def n_samples(self) -> int:
        return self.data.shape[0]

    @property
    def n_features(self) -> int:
        return self.data.shape[1]


class DataPreprocessor:
    """Preprocesses mixed-type data for DP model training.

    Handles:
    - Missing value imputation
    - Categorical encoding (ordinal)
    - Feature scaling
    - Train/test splitting
    """

    def __init__(
        self,
        categorical_columns: list[int] | None = None,
        continuous_columns: list[int] | None = None,
    ):
        """Initialize preprocessor.

        Args:
            categorical_columns: Indices of categorical columns.
            continuous_columns: Indices of continuous columns.
        """
        self.categorical_columns = categorical_columns or []
        self.continuous_columns = continuous_columns or []
        self._label_encoders: dict[int, LabelEncoder] = {}
        self._scaler = StandardScaler()
        self._fitted = False

    def _has_nan(self, col_data: np.ndarray) -> np.ndarray:
        """Check for NaN values safely across numeric and object dtypes.

        Args:
            col_data: A 1-D column array.

        Returns:
            Boolean mask where True means "not NaN" (valid).
        """
        if col_data.dtype.kind in ("O", "U", "S"):
            # Object/string columns — no numeric NaN concept
            return ~pd.isna(col_data)
        else:
            # Numeric columns
            return ~np.isnan(col_data.astype(float))

    def fit(self, data: np.ndarray) -> DataPreprocessor:
        """Fit preprocessor to data.

        Automatically detects column types if not specified.

        Args:
            data: Input data of shape (n_samples, n_features).

        Returns:
            Self for chaining.
        """
        n_features = data.shape[1]

        if not self.categorical_columns and not self.continuous_columns:
            # Auto-detect
            for col_idx in range(n_features):
                col_data = data[:, col_idx]
                valid_mask = self._has_nan(col_data)
                unique_vals = np.unique(col_data[valid_mask])
                if len(unique_vals) <= 20 or col_data.dtype.kind in ("O", "U"):
                    self.categorical_columns.append(col_idx)
                else:
                    self.continuous_columns.append(col_idx)

        # Fit label encoders for categorical columns
        for col_idx in self.categorical_columns:
            col_data = data[:, col_idx]
            # Handle NaN in categoricals
            valid_mask = ~pd.isna(col_data).flatten() if col_data.ndim > 1 else ~pd.isna(col_data)
            valid_data = col_data[valid_mask] if isinstance(valid_mask, np.ndarray) else col_data

            if len(np.unique(valid_data)) > 0:
                encoder = LabelEncoder()
                encoder.fit(valid_data.astype(str))
                self._label_encoders[col_idx] = encoder

        # Fit scaler for continuous columns
        if self.continuous_columns:
            cont_data = np.column_stack([
                data[:, col_idx].astype(float)
                for col_idx in self.continuous_columns
            ])
            # Impute NaN with mean
            col_mean = np.nanmean(cont_data, axis=0)
            inds = np.where(np.isnan(cont_data))
            cont_data[inds] = np.take(col_mean, inds[1])
            self._scaler.fit(cont_data)

        self._fitted = True
        return self

    def transform(self, data: np.ndarray) -> np.ndarray:
        """Transform data to model-ready format.

        Args:
            data: Input data.

        Returns:
            Transformed data (all numerical, NaN-free).
        """
        if not self._fitted:
            raise RuntimeError("Preprocessor not fitted. Call fit() first.")

        n_features = data.shape[1]
        output = np.zeros_like(data, dtype=float)

        # Transform categorical columns
        for col_idx in self.categorical_columns:
            if col_idx in self._label_encoders:
                encoder = self._label_encoders[col_idx]
                col_data = data[:, col_idx]
                # Handle unseen categories and NaN
                str_data = col_data.astype(str)
                str_data = np.where(
                    pd.isna(col_data), "MISSING", str_data
                )
                # Map unseen categories to a default
                for i in range(len(str_data)):
                    if str_data[i] not in encoder.classes_:
                        str_data[i] = encoder.classes_[0]
                output[:, col_idx] = encoder.transform(str_data)

        # Transform continuous columns
        if self.continuous_columns:
            cont_data = np.column_stack([
                data[:, col_idx].astype(float)
                for col_idx in self.continuous_columns
            ])
            # Impute NaN with fitted mean
            col_mean = self._scaler.mean_
            for i, col_idx in enumerate(self.continuous_columns):
                nan_mask = np.isnan(cont_data[:, i])
                if nan_mask.any():
                    cont_data[nan_mask, i] = col_mean[i]

            cont_scaled = self._scaler.transform(cont_data)
            for i, col_idx in enumerate(self.continuous_columns):
                output[:, col_idx] = cont_scaled[:, i]

        return output

    def fit_transform(self, data: np.ndarray) -> np.ndarray:
        """Fit and transform in one step.

        Args:
            data: Input data.

        Returns:
            Transformed data.
        """
        return self.fit(data).transform(data)


def load_benchmark_dataset(name: str) -> Dataset | None:
    """Load a standard benchmark dataset for DP evaluation.

    Supported datasets:
    - 'adult': Adult Census Income (classification)
    - 'credit': Credit Card Default (classification)
    - 'covertype': Forest Covertype (classification, sampled)

    Args:
        name: Dataset name.

    Returns:
        Dataset object or None if unavailable.
    """
    name = name.lower().strip()

    if name == "adult":
        return _load_adult()
    elif name == "credit":
        return _load_credit()
    elif name == "covertype":
        return _load_covertype()
    else:
        logger.warning(f"Unknown dataset: {name}. Supported: adult, credit, covertype")
        return None


def _load_adult() -> Dataset | None:
    """Load Adult Census dataset.

    Returns:
        Dataset with income prediction target (column 14).
    """
    try:
        url = (
            "https://archive.ics.uci.edu/ml/machine-learning-databases/"
            "adult/adult.data"
        )
        columns = [
            "age", "workclass", "fnlwgt", "education", "education-num",
            "marital-status", "occupation", "relationship", "race",
            "sex", "capital-gain", "capital-loss", "hours-per-week",
            "native-country", "income",
        ]
        df = pd.read_csv(
            url,
            names=columns,
            sep=r",\s*",
            engine="python",
            na_values=["?"],
        )
        # Drop missing
        df = df.dropna().reset_index(drop=True)

        # Binary encoding of target
        df["income"] = (df["income"] == ">50K").astype(int)

        cat_cols = [1, 3, 5, 6, 7, 8, 9, 13]
        cont_cols = [0, 2, 4, 10, 11, 12]

        data = df.values.astype(object)
        return Dataset(
            data=data,
            target_column=14,
            column_names=columns,
            categorical_columns=cat_cols,
            continuous_columns=cont_cols,
            name="adult",
        )
    except Exception as e:
        logger.warning(f"Failed to load Adult dataset: {e}")
        return None


def _load_credit() -> Dataset | None:
    """Load Credit Card Default dataset.

    Returns:
        Dataset with default payment target (column 0 after processing).
    """
    try:
        url = (
            "https://archive.ics.uci.edu/ml/machine-learning-databases/"
            "00350/default%20of%20credit%20card%20clients.xls"
        )
        df = pd.read_excel(url, header=1)
        # Target is 'default payment next month' (last column)
        target_col = df.columns[-1]
        cat_cols = [1, 2, 3, 4, 5, 6, 7]  # Gender, education, marital, etc.
        cont_cols = [i for i in range(df.shape[1]) if i not in cat_cols and i != target_col]

        data = df.values.astype(object)
        return Dataset(
            data=data,
            target_column=df.columns.get_loc(target_col),
            column_names=list(df.columns),
            categorical_columns=cat_cols,
            continuous_columns=cont_cols,
            name="credit",
        )
    except Exception as e:
        logger.warning(f"Failed to load Credit dataset: {e}")
        return None


def _load_covertype() -> Dataset | None:
    """Load Forest Covertype dataset (sampled, first 10k rows).

    Returns:
        Dataset with covertype target (column 54).
    """
    try:
        url = (
            "https://archive.ics.uci.edu/ml/machine-learning-databases/"
            "covtype/covtype.data.gz"
        )
        df = pd.read_csv(url, header=None, compression="gzip")
        # Sample first 10k for faster experiments
        df = df.iloc[:10000].reset_index(drop=True)

        # All columns except last are continuous
        cont_cols = list(range(df.shape[1] - 1))
        cat_cols = []

        data = df.values.astype(object)
        return Dataset(
            data=data,
            target_column=df.shape[1] - 1,
            column_names=[f"feature_{i}" for i in range(df.shape[1] - 1)] + ["covertype"],
            categorical_columns=cat_cols,
            continuous_columns=cont_cols,
            name="covertype",
        )
    except Exception as e:
        logger.warning(f"Failed to load Covertype dataset: {e}")
        return None
