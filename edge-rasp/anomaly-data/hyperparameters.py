import pandas as pd
import numpy as np

from sklearn.neighbors import NearestNeighbors
from matplotlib import pyplot as plt
from sklearn.preprocessing import StandardScaler


def main():
    # https://medium.com/@tarammullin/dbscan-parameter-estimation-ff8330e3a3bd
    # we try to determine epsilon following a scientific approach that creates a knee-plot

    dataset = pd.read_csv(filepath_or_buffer="data.csv", sep=";", usecols=[2,3,4])

    scaler = StandardScaler()
    scaler = scaler.fit_transform(dataset)
    neighbors = NearestNeighbors(n_neighbors=6)  # 2 * dimension
    neighbors_fit = neighbors.fit(scaler)
    distances, indices = neighbors_fit.kneighbors(scaler)

    distances = np.sort(distances, axis=0)
    distances = distances[:, 1]

    fig, (ax1, ax2) = plt.subplots(2)
    fig.suptitle('Hyperparameter determination')
    ax1.plot(scaler)
    ax2.plot(distances)
    plt.show()


if __name__ == '__main__':
    main()
