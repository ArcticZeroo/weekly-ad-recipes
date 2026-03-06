import React from 'react';
import { NavLink } from 'react-router-dom';
import { classNames } from '../../util/classnames.ts';
import styles from './nav-bar.module.scss';

export const NavBar: React.FC = () => {
    return (
        <nav className={styles.navBar}>
            <NavLink
                to="/"
                className={({ isActive }) => classNames(styles.title, isActive && styles.activeLink)}
                end
            >
                Weekly Ad Recipes
            </NavLink>
        </nav>
    );
};
